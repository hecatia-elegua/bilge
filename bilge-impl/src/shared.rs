use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, abort};
use quote::{ToTokens, quote};
use syn::{DeriveInput, LitInt, Expr, Variant, Type, Lit, ExprLit, Meta, Data, Attribute};

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
pub(crate) fn analyze_derive(derive_input: &DeriveInput, try_from: bool) -> (&syn::Data, TokenStream, &Ident, BitSize, Option<&Variant>) {
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

    let fallback = fallback_variant(data);
    if fallback.is_some() && try_from {
        abort_call_site!("fallback is not allowed with `TryFromBits`"; help = "use `#[derive(FromBits)]` or remove this `#[fallback]`")
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

    (data, arb_int, ident, bitsize, fallback)
}

// If we want to support bitsize(u4) besides bitsize(4), do that here.
pub fn bitsize_and_arbitrary_int_from(bitsize_arg: TokenStream) -> (BitSize, TokenStream) {
    let bitsize: LitInt = syn::parse2(bitsize_arg.clone()).unwrap_or_else(|_|
        abort!(bitsize_arg, "attribute value is not a number"; help = "you need to define the size like this: `#[bitsize(32)]`")
    );
    // without postfix
    let bitsize = bitsize.base10_parse().unwrap_or_else(|_|
        abort!(bitsize_arg, "attribute value is not a valid number"; help = "currently, numbers from 1 to {} are allowed", MAX_STRUCT_BIT_SIZE)
    );
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
pub fn enum_fills_bitsize(bitsize: u8, variants_count: usize) -> bool {
    let max_variants_count = 2u128.saturating_pow(bitsize as u32);
    variants_count as u128 == max_variants_count
}

#[inline]
pub fn unreachable<T, U>(_: T) -> U {
    unreachable!("should have already been validated")
}

fn fallback_variant(data: &Data) -> Option<&Variant> {
    match data {
        Data::Enum(enum_data) => {
            let mut variants_with_fallback = enum_data
                .variants
                .iter()
                .filter(|variant| variant.attrs.iter().any(is_fallback_attribute));

            let variant = variants_with_fallback.next();

            if variants_with_fallback.next().is_some() {
                abort_call_site!("only one enum variant may be fallback"; help = "remove #[fallback] attributes until you only have one");
            } else {
                variant
            }
        }
        Data::Struct(struct_data) => {
            let mut field_attrs = struct_data.fields.iter().flat_map(|field| &field.attrs);
            
            if field_attrs.any(is_fallback_attribute) {
                abort_call_site!("the attribute `fallback` is only applicable to enums"; help = "remove all `#[fallback]` from this struct")
            } else {
                None
            }
        }
        _ => unreachable(())
    }
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

pub(crate) struct EnumVariantValueAssigner {
    bitsize: u8,
    next_expected_assignment: u128,
}

impl EnumVariantValueAssigner {
    pub fn new(bitsize: u8) -> EnumVariantValueAssigner {
        EnumVariantValueAssigner { bitsize, next_expected_assignment: 0 }
    }
    
    fn max_value(&self) -> u128 {
        2u128.saturating_pow(self.bitsize as u32) - 1
    }

    fn value_from_discriminant(&self, variant: &Variant) -> Option<u128> {
        let discriminant = variant.discriminant.as_ref()?;
        let discriminant_expr = &discriminant.1;
        let variant_name = &variant.ident;

        let Expr::Lit(ExprLit { lit: Lit::Int(int), .. }) = discriminant_expr else {
            abort!(
                discriminant_expr, 
                "variant `{}` is not a number", variant_name; 
                help = "only literal integers currently supported"
            )
        };
    
        let discriminant_value: u128 = int.base10_parse().unwrap_or_else(unreachable);
        if discriminant_value > self.max_value() {
            abort_call_site!("Value of variant {} exceeds the given number of bits", variant_name)
        }

        Some(discriminant_value)
    }

    pub fn assign(&mut self, variant: &Variant) -> u128 {
        let value = self.value_from_discriminant(variant).unwrap_or(self.next_expected_assignment);
        self.next_expected_assignment = value + 1;
        value
    }
}