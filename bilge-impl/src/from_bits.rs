use proc_macro2::{TokenStream, Ident};
use quote::quote;
use syn::{DeriveInput, Data, Variant};
use crate::shared::{self, unreachable, analyze_enum_derive, analyze_derive, generate_enum};

pub(super) fn from_bits(item: TokenStream) -> TokenStream {
    let derive_input = parse(item);
    let try_from = false;
    let (derive_data, arb_int, name, internal_bitsize, derive_impl) = analyze_derive(&derive_input, try_from);
    let expanded = match derive_data {
        Data::Struct(_) => {
            generate_struct(arb_int, name)
        },
        Data::Enum(ref enum_data) => {
            let variants = enum_data.variants.iter();
            let match_arms = analyze_enum_derive(variants, name, internal_bitsize, &derive_impl);
            generate_enum(arb_int, name, match_arms, &derive_impl)
        },
        _ => unreachable(()),
    };
    let fallback_variant = derive_impl.into_fallback_variant(); 
    generate_common(expanded, name, fallback_variant)
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
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

fn generate_common(expanded: TokenStream, type_name: &Ident, _fallback_variant: Option<Variant>) -> TokenStream {
    // TODO: if fallback_variant.is_some(), an assert should not be generated
    quote! {
        #expanded

        const _: () = assert!(#type_name::FILLED, "implementing FromBits on bitfields with unfilled bits is forbidden");
    }
}
