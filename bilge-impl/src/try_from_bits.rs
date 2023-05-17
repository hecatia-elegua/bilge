use proc_macro2::{TokenStream, Ident};
use quote::quote;
use syn::{DeriveInput, Data, punctuated::Iter, Variant, Type, Fields};

use crate::shared::{self, BitSize, unreachable};

pub(super) fn try_from_bits(item: TokenStream) -> TokenStream {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, internal_bitsize) = analyze(&derive_input);
    match derive_data {
        Data::Struct(ref data) => {
            codegen_struct(arb_int, name, &data.fields)
        },
        Data::Enum(ref enum_data) => {
            let variants = enum_data.variants.iter();
            let match_arms = analyze_enum(variants, name, internal_bitsize);
            codegen_enum(arb_int, name, match_arms)
        },
        _ => unreachable(()),
    }
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> (&syn::Data, TokenStream, &Ident, BitSize) {
    shared::analyze_derive(derive_input, true)
}

fn analyze_enum(variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize) -> (Vec<TokenStream>, Vec<TokenStream>) {
    shared::analyze_enum_derive(variants, name, internal_bitsize, true)
}

fn codegen_enum(arb_int: TokenStream, enum_type: &Ident, match_arms: (Vec<TokenStream>, Vec<TokenStream>)) -> TokenStream {
    let (from_int_match_arms, to_int_match_arms) = match_arms;

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    let from_enum_impl = shared::generate_from_enum_impl(&arb_int, enum_type, to_int_match_arms, &const_);
    quote! {
        impl #const_ ::core::convert::TryFrom<#arb_int> for #enum_type {
            type Error = #arb_int;

            fn try_from(number: #arb_int) -> ::core::result::Result<Self, Self::Error> {
                match number.value() {
                    #( #from_int_match_arms )*
                    i => Err(#arb_int::new(i)),
                }
            }
        }

        // this other direction is needed for get/set/new
        #from_enum_impl
    }
}

fn generate_field_check(ty: &Type) -> TokenStream {
    // Yes, this is hacky module management.
    crate::bitsize_internal::struct_gen::generate_getter_inner(ty, false)
}

fn codegen_struct(arb_int: TokenStream, struct_type: &Ident, fields: &Fields) -> TokenStream {
    let is_ok: TokenStream = fields.iter()
        .map(|field| {
            let ty = &field.ty;
            if shared::is_always_filled(ty) {
                let size = shared::generate_field_bitsize(ty);
                quote! { {
                    // we still need to shift by the element's size
                    let size = #size;
                    cursor >>= size;
                    true
                } }
            } else {
                generate_field_check(ty)
            }
        })
        .reduce(|acc, next| quote!((#acc && #next)))
        // `Struct {}` would be handled like this:
        .unwrap_or_else(|| quote!(true));

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    quote! {
        impl #const_ ::core::convert::TryFrom<#arb_int> for #struct_type {
            type Error = #arb_int;
            
            // validates all values, which means enums, even in inner structs (TODO: and reserved fields?)
            fn try_from(value: #arb_int) -> ::core::result::Result<Self, Self::Error> {
                type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
                type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;

                // cursor starts at value's first field
                let mut cursor = value.value();

                let is_ok: bool = {#is_ok};

                if is_ok {
                    Ok(Self { value })
                } else {
                    Err(value)
                }
            }
        }

        impl #const_ ::core::convert::From<#struct_type> for #arb_int {
            fn from(struct_value: #struct_type) -> Self {
                struct_value.value
            }
        }

        // TODO: this is relevant to non_exhaustive and doesn't need to be forbidden
        // const _: () = assert!(!#struct_type::FILLED, "implementing TryFromBits on bitfields with filled bits is unneccessary"); 
    }
}
