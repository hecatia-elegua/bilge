use proc_macro2::{TokenStream, Ident};
use quote::quote;
use syn::{DeriveInput, Data, punctuated::Iter, Variant};

use crate::shared::{self, BitSize, unreachable};

pub(super) fn from_bits(item: TokenStream) -> TokenStream {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, internal_bitsize) = analyze(&derive_input);
    let expanded = match derive_data {
        Data::Struct(_) => {
            generate_struct(arb_int, name)
        },
        Data::Enum(ref enum_data) => {
            let variants = enum_data.variants.iter();
            let match_arms = analyze_enum(variants, name, internal_bitsize);
            generate_enum(arb_int, name, match_arms)
        },
        _ => unreachable(()),
    };
    generate_common(expanded, name)
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> (&syn::Data, TokenStream, &Ident, BitSize) {
    shared::analyze_derive(derive_input, false)
}

fn analyze_enum(variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize) -> (Vec<TokenStream>, Vec<TokenStream>) {
    shared::analyze_enum_derive(variants, name, internal_bitsize, false)
}

fn generate_enum(arb_int: TokenStream, enum_type: &Ident, match_arms: (Vec<TokenStream>, Vec<TokenStream>)) -> TokenStream {
    let (from_int_match_arms, to_int_match_arms) = match_arms;

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    let from_enum_impl = shared::generate_from_enum_impl(&arb_int, enum_type, to_int_match_arms, &const_);

    quote! {
        impl #const_ ::core::convert::From<#arb_int> for #enum_type {
            fn from(number: #arb_int) -> Self {
                match number.value() {
                    #( #from_int_match_arms )*
                    // constness: unreachable!() is not const yet
                    _ => panic!("unreachable: arbitrary_int already validates that this is unreachable")
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
