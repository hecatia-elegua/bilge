use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Variant, punctuated::Iter};

use crate::shared::discriminant::EnumDiscriminant;
use crate::shared::{
    self, BitSize, binary_segments, discriminant, discriminant_assigner::DiscriminantAssigner, discriminant_at, fallback::Fallback,
    parse_enum_discriminant, place_struct_fields, unreachable,
};

pub(crate) fn binary(item: TokenStream) -> manyhow::Result {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, bitsize, fallback) = analyze(&derive_input)?;

    match derive_data {
        Data::Struct(data) => Ok(generate_struct_binary_impl(name, &data.fields, bitsize)),
        Data::Enum(data) => match parse_enum_discriminant(&derive_input.attrs)? {
            Some(EnumDiscriminant::Type(disc)) => generate_external_binary_impl(name, data.variants.iter(), arb_int, &disc),
            tag => {
                let disc = match tag {
                    Some(EnumDiscriminant::At(disc)) => {
                        disc.validate(bitsize as usize)?;
                        Some(disc)
                    }
                    _ => None,
                };
                generate_enum_binary_impl(name, data.variants.iter(), arb_int, bitsize, fallback, disc.as_ref())
            }
        },
        _ => unreachable(()),
    }
}

fn generate_struct_binary_impl(struct_name: &Ident, fields: &Fields, declared_bitsize: BitSize) -> TokenStream {
    let layout = place_struct_fields(fields, declared_bitsize as usize).unwrap_or_else(|_| unreachable(()));
    let declared = declared_bitsize as usize;

    // fields (and holes) are printed from most significant to least significant
    let writes = binary_segments(&layout, declared).into_iter().rev().map(|(offset, width)| {
        quote! {
            let field_size = #width;
            if field_size != 0 {
                if started {
                    ::core::write!(f, "_")?;
                }
                started = true;
                let field_mask = mask >> (struct_size - field_size);
                let extracted = field_mask & (self.value >> #offset);
                ::core::write!(f, "{:0width$b}", extracted, width = field_size)?;
            }
        }
    });

    quote! {
        impl ::core::fmt::Binary for #struct_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let struct_size = <#struct_name as Bitsized>::BITS;
                let mask = <#struct_name as Bitsized>::MAX;
                let mut started = false;
                #(#writes)*
                Ok(())
            }
        }
    }
}

fn generate_external_binary_impl(
    enum_name: &Ident, variants: Iter<Variant>, arb_int: TokenStream, disc: &discriminant::Discriminant,
) -> manyhow::Result<TokenStream> {
    let mut assigner = DiscriminantAssigner::new(disc.assigner_width());
    let mut arms = Vec::new();
    for variant in variants {
        let _ = assigner.assign_unsuffixed(variant)?;
        let payload_ty = discriminant_at::variant_payload_ty(variant)?;
        arms.push(discriminant::payload_only_to_int_arm(enum_name, &variant.ident, payload_ty, &arb_int));
    }

    let body = if arms.is_empty() {
        quote! { Ok(()) }
    } else {
        quote! {
            let value = match self {
                #( #arms )*
            };
            ::core::write!(f, "{:0width$b}", value, width = <#enum_name as Bitsized>::BITS)
        }
    };

    Ok(quote! {
        impl ::core::fmt::Binary for #enum_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    })
}

fn generate_enum_binary_impl(
    enum_name: &Ident, variants: Iter<Variant>, arb_int: TokenStream, bitsize: BitSize, fallback: Option<Fallback>,
    disc: Option<&discriminant_at::DiscriminantAt>,
) -> manyhow::Result {
    let to_int_match_arms = generate_to_int_match_arms(variants, enum_name, bitsize, arb_int, fallback, disc)?;

    let body = if to_int_match_arms.is_empty() {
        quote! { Ok(()) }
    } else {
        quote! {
            let value = match self {
                #( #to_int_match_arms )*
            };
            ::core::write!(f, "{:0width$b}", value, width = <#enum_name as Bitsized>::BITS)
        }
    };

    Ok(quote! {
        impl ::core::fmt::Binary for #enum_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    })
}

/// generates the arms for an (infallible) conversion from an enum to the enum's underlying arbitrary_int
fn generate_to_int_match_arms(
    variants: Iter<Variant>, enum_name: &Ident, bitsize: BitSize, arb_int: TokenStream, fallback: Option<Fallback>,
    disc: Option<&discriminant_at::DiscriminantAt>,
) -> manyhow::Result<Vec<TokenStream>> {
    let is_value_fallback = |variant_name| {
        if let Some(Fallback::WithValue(name)) = &fallback {
            variant_name == name
        } else {
            false
        }
    };

    let fill_width = disc.map(|d| d.width as u8).unwrap_or(bitsize);
    let mut assigner = DiscriminantAssigner::new(fill_width);

    variants
        .map(|variant| -> manyhow::Result<TokenStream> {
            let variant_name = &variant.ident;
            let variant_value = assigner.assign_unsuffixed(variant)?;

            Ok(if is_value_fallback(variant_name) {
                quote! { #enum_name::#variant_name(number) => *number, }
            } else if let Some(disc) = disc {
                let payload_ty = discriminant_at::variant_payload_ty(variant)?;
                discriminant_at::payload_to_int_arm(disc, bitsize as usize, enum_name, variant_name, payload_ty, &variant_value, &arb_int)
            } else {
                shared::to_int_match_arm(enum_name, variant_name, &arb_int, variant_value)
            })
        })
        .collect()
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> manyhow::Result<(&Data, TokenStream, &Ident, BitSize, Option<Fallback>)> {
    shared::analyze_derive(derive_input, false)
}
