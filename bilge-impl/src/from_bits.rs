use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, abort};
use quote::quote;
use syn::{Fields, DeriveInput, Data, punctuated::Iter, Variant};
use crate::shared::{fallback::Fallback, self, BitSize, unreachable, variant_value_assigner::EnumVariantValueAssigner, enum_fills_bitsize};

pub(super) fn from_bits(item: TokenStream) -> TokenStream {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, internal_bitsize, fallback) = analyze(&derive_input);
    let expanded = match derive_data {
        Data::Struct(_) => {
            generate_struct(arb_int, name)
        },
        Data::Enum(ref enum_data) => {
            let variants = enum_data.variants.iter();
            let match_arms = analyze_enum(variants, name, internal_bitsize, fallback.as_ref());
            generate_enum(arb_int, name, match_arms, fallback)
        },
        _ => unreachable(()),
    };
    generate_common(expanded, name)
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> (&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>) {
    shared::analyze_derive(derive_input, false)
}

fn analyze_enum(variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize, fallback: Option<&Fallback>) -> (Vec<TokenStream>, Vec<TokenStream>) {
    validate_enum_variants(variants.clone(), fallback);
    
    let enum_is_filled = enum_fills_bitsize(internal_bitsize, variants.len());
    if !enum_is_filled && fallback.is_none() {
        abort_call_site!("enum doesn't fill its bitsize"; help = "you need to use `#[derive(TryFromBits)]` instead, or specify one of the variants as #[fallback]")
    }
    if enum_is_filled && fallback.is_some() {
        abort_call_site!("enum fills its bitsize but has fallback variant"; help = "remove `#[fallback]` from this enum")
    }

    let mut value_assigner = EnumVariantValueAssigner::new(internal_bitsize);

    variants.map(|variant| {
        let variant_name = &variant.ident;
        let variant_value = value_assigner.assign_unsuffixed(variant);

        let from_int_match_arm = if is_ident_of_fallback(fallback, variant_name) {
            // this value will be handled by the catch-all arm
            quote!()
        } else {
            quote! { #variant_value => Self::#variant_name, }
        };

        let to_int_match_arm = if is_ident_of_fallback_with_value(fallback, variant_name) {
            quote! { #name::#variant_name(number) => number, } 
        } else {
            quote! { #name::#variant_name => Self::new(#variant_value), }
        };

        (from_int_match_arm, to_int_match_arm)
    }).unzip()
}

fn generate_enum(arb_int: TokenStream, enum_type: &Ident, match_arms: (Vec<TokenStream>, Vec<TokenStream>), fallback: Option<Fallback>) -> TokenStream {
    let (from_int_match_arms, to_int_match_arms) = match_arms;

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    let from_enum_impl = shared::generate_from_enum_impl(&arb_int, enum_type, to_int_match_arms, &const_);

    let catch_all_arm = match fallback {
        Some(Fallback::WithValue(fallback_ident)) => quote! {
            _ => Self::#fallback_ident(number),
        },
        Some(Fallback::Unit(fallback_ident)) => quote! {
            _ => Self::#fallback_ident,
        },
        None => quote! {
            // constness: unreachable!() is not const yet
            _ => panic!("unreachable: arbitrary_int already validates that this is unreachable")
        },
    };

    quote! {
        impl #const_ ::core::convert::From<#arb_int> for #enum_type {
            fn from(number: #arb_int) -> Self {
                match number.value() {
                    #( #from_int_match_arms )*
                    #catch_all_arm
                }
            }
        }
        #from_enum_impl
    }
}

fn generate_struct(arb_int: TokenStream, struct_type: &Ident) -> TokenStream {
    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    quote! {
        impl #const_ ::core::convert::From<#arb_int> for #struct_type {
            fn from(value: #arb_int) -> Self {
                Self { value }
            }
        }
        impl #const_ ::core::convert::From<#struct_type> for #arb_int {
            fn from(value: #struct_type) -> Self {
                value.value
            }
        }
    }
}

fn generate_common(expanded: TokenStream, type_name: &Ident) -> TokenStream {
    quote! {
        #expanded

        const _: () = assert!(#type_name::FILLED, "implementing FromBits on bitfields with unfilled bits is forbidden");
    }
}

fn validate_enum_variants(variants: Iter<Variant>, fallback: Option<&Fallback>) {
    for variant in variants {
        // we've already validated the correctness of the fallback variant, and that there's at most one such variant.
        // this means we can safely skip a fallback variant if we find one.
        if is_ident_of_fallback(fallback, &variant.ident) {
            continue;
        }

        if !matches!(variant.fields, Fields::Unit) {
            let help_message = if fallback.is_some() {
                "change this variant to a unit"
            } else {
                "add a fallback variant or change this variant to a unit"
            };
            abort!(variant, "FromBits only supports unit variants for variants without fallback"; help = help_message);
        }
    }
}

fn is_ident_of_fallback(fallback: Option<&Fallback>, ident: &Ident) -> bool {
    match fallback {
        Some(Fallback::Unit(fallback_ident) | Fallback::WithValue(fallback_ident)) => ident == fallback_ident,
        _ => false,
    }
}

fn is_ident_of_fallback_with_value(fallback: Option<&Fallback>, ident: &Ident) -> bool {
    match fallback {
        Some(Fallback::WithValue(fallback_ident)) => ident == fallback_ident,
        _ => false,
    }
}