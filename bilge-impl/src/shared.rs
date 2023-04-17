use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, emit_call_site_warning, abort};
use quote::{ToTokens, quote};
use syn::{DeriveInput, LitInt, Expr, punctuated::Iter, Variant, Type, Lit, ExprLit, Meta};

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
pub(crate) fn analyze_derive(derive_input: &DeriveInput, try_from: bool) -> (&syn::Data, TokenStream, &Ident, BitSize) {
    let DeriveInput { 
        attrs,
        ident,
        // generics,
        data,
        ..
    } = derive_input;

    if !try_from {
        if attrs.iter().any(|attr| 
            matches!(&attr.meta, Meta::Path(path) if path.to_token_stream().to_string().contains("non_exhaustive"))
        ) {
            abort_call_site!("Item can't be FromBits and non_exhaustive"; help = "remove #[non_exhaustive] or derive(FromBits) here")
        }
    } else {
        // currently not allowed, would need some thinking:
        if let syn::Data::Struct(_) = data {
            if attrs.iter().any(|attr| 
                matches!(&attr.meta, Meta::Path(path) if path.to_token_stream().to_string().contains("non_exhaustive"))
            ) {
                abort_call_site!("Using #[non_exhaustive] on structs is currently not supported"; help = "open an issue on our repository if needed")
            }
        }
    }
    // parsing the #[bitsize_internal(num)] attribute macro
    let internal_bitsize_attr = attrs.iter().find_map(|attr| {
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
    let (bitsize, arb_int) = bitsize_and_arbitrary_int_from(internal_bitsize_attr);

    (data, arb_int, ident, bitsize)
}

// If we want to support bitsize(u4) besides bitsize(4), do that here.
pub fn bitsize_and_arbitrary_int_from(bitsize_attr: TokenStream) -> (BitSize, TokenStream) {
    let bitsize: LitInt = syn::parse2(bitsize_attr.clone()).unwrap_or_else(|_|
        abort!(bitsize_attr, "attribute value is not a number"; help = "you need to define the size like this: `#[bitsize(32)]`")
    );
    // without postfix
    let bitsize = bitsize.base10_parse().unwrap_or_else(|_|
        abort!(bitsize_attr, "attribute value is not a valid number"; help = "currently, numbers from 1 to {} are allowed", MAX_STRUCT_BIT_SIZE)
    );
    let arb_int = syn::parse_str(&format!("u{bitsize}")).unwrap_or_else(unreachable);
    (bitsize, arb_int)
}

pub fn analyze_field_bitsize(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            tuple.elems.iter().map(analyze_field_bitsize)
                .reduce(|acc, next| quote!((#acc + #next)))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        },
        Array(array) => {
            let elem_bitsize = analyze_field_bitsize(&array.elem);
            let len_expr = &array.len;
            quote!((#elem_bitsize * #len_expr))
        },
        Path(_) => {
            quote!(<#ty as Bitsized>::BITS)
        },
        _ => unreachable(()),
    }
}

// allow since we want `if try_from` blocks to stand out
#[allow(clippy::collapsible_else_if)]
pub(crate) fn analyze_enum_derive(variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize, try_from: bool) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let variants_count = variants.len();
    // in enums, internal_bitsize <= 64; u64::MAX + 1 = u128
    let max_variants_count = 1u128 << internal_bitsize;

    // Verify if the enum fills its bitsize, depending on which derive impl we are in.
    // Verifying that the value doesn't exceed max_variants_count is done further down.
    if try_from {
        if variants_count as u128 == max_variants_count {
            emit_call_site_warning!("enum fills its bitsize"; help = "you can use `#[derive(FromBits)]` instead, rust will provide `TryFrom` for you (so you don't necessarily have to update call-sites)");
        }
    } else {
        // semantically the same as #[non_exhaustive]
        if variants_count as u128 != max_variants_count {
            abort_call_site!("enum doesn't fill its bitsize"; help = "you need to use `#[derive(TryFromBits)]` instead")
        }
    }    

    let mut next_variant_value = 0;
    variants.map(|variant| {
        let variant_name = &variant.ident;
        let variant_value: u128 = match variant.discriminant.as_ref() {
            Some(d) => {
                let discriminant_expr = &d.1;
                match discriminant_expr {
                    Expr::Lit(ExprLit { lit: Lit::Int(int), .. }) => int.base10_parse().unwrap_or_else(unreachable),
                    _ => abort!(
                        discriminant_expr, "variant `{}` is not a number", variant_name;
                        help = "only literal integers currently supported"
                    )
                }
            }
            None => next_variant_value,
        };
        next_variant_value = variant_value + 1;

        if variant_value >= max_variants_count {
            abort_call_site!("Value {} exceeds the given number of bits", variant_name);
        }

        // might be useful for not generating "1u128 -> Self::Variant"
        let variant_value: Expr = syn::parse_str(&variant_value.to_string()).unwrap_or_else(unreachable);

        let from_int_match_arm = if try_from {
            quote! {
                #variant_value => Ok(Self::#variant_name),
            }
        }  else {
            quote! {
                #variant_value => Self::#variant_name,
            }
        };

        let to_int_match_arm = quote! {
            #name::#variant_name => Self::new(#variant_value),
        };

        (from_int_match_arm, to_int_match_arm)
    }).unzip()
}

pub(crate) fn generate_from_enum_impl(arb_int: &TokenStream, enum_type: &Ident, to_int_match_arms: Vec<TokenStream>) -> TokenStream {
    quote! {
        impl const ::core::convert::From<#enum_type> for #arb_int {
            fn from(enum_value: #enum_type) -> Self {
                match enum_value {
                    #( #to_int_match_arms )*
                    // constness: unreachable!() is not const yet
                    _ => panic!("unreachable: arbitrary_int already validates that this is unreachable")
                }
            }
        }
    }
}

/// Filters fields which are always `FILLED`, meaning all bit-patterns are possible,
/// meaning they are (should be) From<uN>, not TryFrom<uN>
/// 
//TODO: We should maybe just rewrite this into something useful or add FILLED into Bitsized impls.
pub fn is_always_filled(ty: &Type) -> bool {
    let ty = ty.to_token_stream().to_string();
    ty.starts_with('u') || ty == "bool"
}


#[inline]
pub fn unreachable<T, U>(_: T) -> U {
    unreachable!("should have already been validated")
}
