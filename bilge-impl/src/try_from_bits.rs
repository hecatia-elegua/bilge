use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{emit_call_site_warning, abort};
use quote::quote;
use syn::{DeriveInput, Data, punctuated::Iter, Variant, Type, Fields};
use crate::shared::{last_ident_of_path, bitsize_from_type_ident};
use crate::shared::{fallback::Fallback, self, BitSize, unreachable, enum_fills_bitsize, discriminant_assigner::DiscriminantAssigner};

pub(super) fn try_from_bits(item: TokenStream) -> TokenStream {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, internal_bitsize, ..) = analyze(&derive_input);
    match derive_data {
        Data::Struct(ref data) => {
            codegen_struct(arb_int, name, &data.fields)
        },
        Data::Enum(ref enum_data) => {
            let variants = enum_data.variants.iter();
            let match_arms = analyze_enum(variants, name, internal_bitsize, &arb_int);
            codegen_enum(arb_int, name, match_arms)
        },
        _ => unreachable(()),
    }
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> (&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>) {
    shared::analyze_derive(derive_input, true)
}

fn analyze_enum(variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize, arb_int: &TokenStream) -> (Vec<TokenStream>, Vec<TokenStream>) {
    validate_enum_variants(variants.clone());

    if enum_fills_bitsize(internal_bitsize, variants.len()) {
        emit_call_site_warning!("enum fills its bitsize"; help = "you can use `#[derive(FromBits)]` instead, rust will provide `TryFrom` for you (so you don't necessarily have to update call-sites)");
    } 

    let mut assigner = DiscriminantAssigner::new(internal_bitsize);
    
    variants.map(|variant| {
        let variant_name = &variant.ident;
        let variant_value = assigner.assign_unsuffixed(variant);

        let from_int_match_arm = quote! {
            #variant_value => Ok(Self::#variant_name),
        };

        let to_int_match_arm = shared::to_int_match_arm(name, variant_name, arb_int, variant_value);

        (from_int_match_arm, to_int_match_arm)
    }).unzip()
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
            let size_from_type = last_ident_of_path(ty).and_then(bitsize_from_type_ident);
            if let Some(size) = size_from_type {
                quote! { {
                    // we still need to shift by the element's size
                    let size = #size;
                    cursor = cursor.wrapping_shr(size as u32);
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
    }
}

fn validate_enum_variants(variants: Iter<Variant>) {
    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            abort!(variant, "TryFromBits only supports unit variants in enums"; help = "change this variant to a unit");
        }
    }
}
