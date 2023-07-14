pub mod fallback;
pub mod discriminant_assigner;

use proc_macro2::{TokenStream, Ident, Literal};
use proc_macro_error::{abort_call_site, abort};
use quote::{ToTokens, quote};
use syn::{DeriveInput, LitInt, Type, Meta, Attribute, Path};
use fallback::{Fallback, fallback_variant};

/// As arbitrary_int is limited to basic rust primitives, the maximum is u128.
/// Is there a true usecase for bitfields above this size?
/// This would also be change-worthy when rust starts supporting LLVM's arbitrary integers.
pub const MAX_STRUCT_BIT_SIZE: u8 = 128;
pub type BitSize = u8;

pub(crate) fn parse_derive(item: TokenStream) -> DeriveInput {
    syn::parse2(item).unwrap_or_else(unreachable)
}

// allow since we want `if try_from` blocks to stand out
#[allow(clippy::collapsible_if)]
pub(crate) fn analyze_derive(derive_input: &DeriveInput, try_from: bool) -> (&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>) {
    let DeriveInput { 
        attrs,
        ident,
        // generics,
        data,
        ..
    } = derive_input;

    if !try_from {
        if attrs.iter().any(is_non_exhaustive_attribute) {
            abort_call_site!("Item can't be FromBits and non_exhaustive"; help = "remove #[non_exhaustive] or derive(FromBits) here")
        }
    } else {
        // currently not allowed, would need some thinking:
        if let syn::Data::Struct(_) = data {
            if attrs.iter().any(is_non_exhaustive_attribute) {
                abort_call_site!("Using #[non_exhaustive] on structs is currently not supported"; help = "open an issue on our repository if needed")
            }
        }
    }

    // parsing the #[bitsize_internal(num)] attribute macro
    let args = attrs.iter().find_map(|attr| {
        if attr.to_token_stream().to_string().contains("bitsize_internal") {
            if let Meta::List(list) = &attr.meta {
                Some(list.tokens.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).unwrap_or_else(|| abort_call_site!("add #[bitsize] attribute above your derive attribute"));
    let (bitsize, arb_int) = bitsize_and_arbitrary_int_from(args);

    let fallback = fallback_variant(data, bitsize);
    if fallback.is_some() && try_from {
        abort_call_site!("fallback is not allowed with `TryFromBits`"; help = "use `#[derive(FromBits)]` or remove this `#[fallback]`")
    }

    (data, arb_int, ident, bitsize, fallback)
}

// If we want to support bitsize(u4) besides bitsize(4), do that here.
pub fn bitsize_and_arbitrary_int_from(bitsize_arg: TokenStream) -> (BitSize, TokenStream) {
    let bitsize: LitInt = syn::parse2(bitsize_arg.clone()).unwrap_or_else(|_|
        abort!(bitsize_arg, "attribute value is not a number"; help = "you need to define the size like this: `#[bitsize(32)]`")
    );
    // without postfix
    let bitsize = bitsize
        .base10_parse()
        .ok()
        .filter(|&n| n != 0 && n <= MAX_STRUCT_BIT_SIZE)
        .unwrap_or_else(|| abort!(bitsize_arg, "attribute value is not a valid number"; help = "currently, numbers from 1 to {} are allowed", MAX_STRUCT_BIT_SIZE));
    let arb_int = syn::parse_str(&format!("u{bitsize}")).unwrap_or_else(unreachable);
    (bitsize, arb_int)
}

pub fn generate_type_bitsize(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            tuple.elems.iter().map(generate_type_bitsize)
                .reduce(|acc, next| quote!((#acc + #next)))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        },
        Array(array) => {
            let elem_bitsize = generate_type_bitsize(&array.elem);
            let len_expr = &array.len;
            quote!((#elem_bitsize * #len_expr))
        },
        Path(_) => {
            quote!(<#ty as Bitsized>::BITS)
        },
        _ => unreachable(()),
    }
}

pub(crate) fn generate_from_enum_impl(arb_int: &TokenStream, enum_type: &Ident, to_int_match_arms: Vec<TokenStream>, const_: &TokenStream) -> TokenStream {
    quote! {
        impl #const_ ::core::convert::From<#enum_type> for #arb_int {
            fn from(enum_value: #enum_type) -> Self {
                match enum_value {
                    #( #to_int_match_arms )*
                }
            }
        }
    }
}

/// Filters fields which are always `FILLED`, meaning all bit-patterns are possible,
/// meaning they are (should be) From<uN>, not TryFrom<uN>
/// 
//TODO: We should maybe just rewrite this into something useful or add FILLED into Bitsized impls.
//otherwise, we could check if there is _not_ a struct or enum here by lower/uppercase first letter
pub fn is_always_filled(ty: &Type) -> bool {
    let ty = ty.to_token_stream().to_string();
    ty.starts_with('u') || ty == "bool"
}

/// in enums, internal_bitsize <= 64; u64::MAX + 1 = u128
/// therefore the bitshift would not overflow.
pub fn enum_fills_bitsize(bitsize: u8, variants_count: usize) -> bool {
    let max_variants_count = 1u128 << bitsize;
    if variants_count as u128 > max_variants_count {
        abort_call_site!("enum overflows its bitsize"; help = "there should only be at most {} variants defined", max_variants_count);
    }
    variants_count as u128 == max_variants_count
}

#[inline]
pub fn unreachable<T, U>(_: T) -> U {
    unreachable!("should have already been validated")
}

pub fn is_attribute(attr: &Attribute, name: &str) -> bool {
    if let Meta::Path(path) = &attr.meta {
        path.is_ident(name)
    } else {
        false
    }
}

fn is_non_exhaustive_attribute(attr: &Attribute) -> bool {
    is_attribute(attr, "non_exhaustive")
}

pub(crate) fn is_fallback_attribute(attr: &Attribute) -> bool {
    is_attribute(attr, "fallback")
}

/// attempts to extract the bitsize from a type token named `uN` or `bool`.
/// should return `Result` instead of `Option`, if we decide to add more descriptive error handling.
/// might consider having this take the type name directly, and fetch the last segment's name at call site
pub fn bitsize_from_type_token(path: &Path) -> Option<BitSize> {
    let last_segment = path.segments.last().unwrap_or_else(|| unreachable(())); //validated by syn analysis
    let type_name = last_segment.ident.to_string();
    
    // there's no need to check that PathArguments is PathArguments::None.
    // if the type name passes the below checks then, in the current namespace, 
    // it can't have generic aguments and is definitely not an Fn trait.

    if type_name == "bool" {
        Some(1)
    } else if let Some(suffix) = type_name.strip_prefix('u') {
        // characters which may appear in this suffix are digits, letters and underscores.
        // parse() will reject letters and underscores, so this should be correct.
        let bitsize = suffix.parse().ok();
        
        // the namespace contains u2 up to u{MAX_STRUCT_BIT_SIZE}. can't make assumptions about larger values
        bitsize.filter(|&n| n <= MAX_STRUCT_BIT_SIZE)
    } else {
        None
    }
}

pub fn to_int_match_arm(enum_name: &Ident, variant_name: &Ident, arb_int: &TokenStream, variant_value: Literal) -> TokenStream {
    quote! { #enum_name::#variant_name => #arb_int::new(#variant_value), }
}