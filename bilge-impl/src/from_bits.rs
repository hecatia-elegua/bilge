use itertools::Itertools;
use manyhow::bail;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type, Variant, punctuated::Iter};

use crate::shared::{
    self, BitSize, discriminant_assigner::DiscriminantAssigner, discriminant_at, enum_fills_bitsize, fallback::Fallback, parse_discriminant_at,
    unreachable,
};

pub(super) fn from_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, internal_bitsize, fallback) = analyze(&derive_input)?;
    let expanded = match &derive_data {
        Data::Struct(struct_data) => generate_struct(arb_int, name, &struct_data.fields),
        Data::Enum(enum_data) => {
            let variants = enum_data.variants.iter();
            let disc = parse_discriminant_at(&derive_input.attrs)?;
            if let Some(disc) = &disc {
                disc.validate(internal_bitsize as usize)?;
            }
            let match_arms = analyze_enum(variants, name, internal_bitsize, fallback.as_ref(), &arb_int, disc.as_ref())?;
            let mut assumes = Vec::new();
            if disc.is_some() {
                for variant in enum_data.variants.iter() {
                    if let Ok(Some(ty)) = discriminant_at::variant_payload_ty(variant) {
                        generate_filled_check_for(ty, &mut assumes);
                    }
                }
            }
            generate_enum(arb_int, name, match_arms, fallback, disc.as_ref(), internal_bitsize, assumes)
        }
        _ => unreachable(()),
    };
    Ok(generate_common(expanded))
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> manyhow::Result<(&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>)> {
    shared::analyze_derive(derive_input, false)
}

fn analyze_enum(
    variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize, fallback: Option<&Fallback>, arb_int: &TokenStream,
    disc: Option<&discriminant_at::DiscriminantAt>,
) -> manyhow::Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    validate_enum_variants(variants.clone(), fallback, disc)?;

    let fill_width = disc.map(|d| d.width as u8).unwrap_or(internal_bitsize);
    let enum_is_filled = enum_fills_bitsize(fill_width, variants.len())?;
    if !enum_is_filled && fallback.is_none() {
        bail!("enum doesn't fill its bitsize"; help = "you need to use `#[derive(TryFromBits)]` instead, or specify one of the variants as #[fallback]")
    }
    if enum_is_filled && fallback.is_some() {
        bail!("enum already has {} variants", variants.len(); help = "remove the `#[fallback]` attribute")
    }

    let mut assigner = DiscriminantAssigner::new(fill_width);

    let is_fallback = |variant_name| {
        if let Some(Fallback::Unit(name) | Fallback::WithValue(name)) = fallback {
            variant_name == name
        } else {
            false
        }
    };

    let is_value_fallback = |variant_name| {
        if let Some(Fallback::WithValue(name)) = fallback {
            variant_name == name
        } else {
            false
        }
    };

    variants
        .map(|variant| -> manyhow::Result<(TokenStream, TokenStream)> {
            let variant_name = &variant.ident;
            let variant_value = assigner.assign_unsuffixed(variant)?;

            if is_fallback(variant_name) {
                let to_int_match_arm = if is_value_fallback(variant_name) {
                    quote! { #name::#variant_name(number) => number, }
                } else if let Some(disc) = disc {
                    discriminant_at::payload_to_int_arm(disc, internal_bitsize as usize, name, variant_name, None, &variant_value, arb_int)
                } else {
                    shared::to_int_match_arm(name, variant_name, arb_int, variant_value)
                };
                return Ok((quote!(), to_int_match_arm));
            }

            if let Some(disc) = disc {
                let payload_ty = discriminant_at::variant_payload_ty(variant)?;
                let from_int_match_arm =
                    discriminant_at::payload_from_int_arm(disc, internal_bitsize as usize, variant_name, payload_ty, &variant_value, false);
                let to_int_match_arm =
                    discriminant_at::payload_to_int_arm(disc, internal_bitsize as usize, name, variant_name, payload_ty, &variant_value, arb_int);
                Ok((from_int_match_arm, to_int_match_arm))
            } else {
                let from_int_match_arm = quote! { #variant_value => Self::#variant_name, };
                let to_int_match_arm = shared::to_int_match_arm(name, variant_name, arb_int, variant_value);
                Ok((from_int_match_arm, to_int_match_arm))
            }
        })
        .collect::<manyhow::Result<Vec<_>>>()
        .map(|arms| arms.into_iter().unzip())
}

fn generate_enum(
    arb_int: TokenStream, enum_type: &Ident, match_arms: (Vec<TokenStream>, Vec<TokenStream>), fallback: Option<Fallback>,
    disc: Option<&discriminant_at::DiscriminantAt>, bitsize: BitSize, assumes: Vec<TokenStream>,
) -> TokenStream {
    let (from_int_match_arms, to_int_match_arms) = match_arms;

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

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
            _ => ::core::panic!("unreachable: arbitrary_int already validates that this is unreachable")
        },
    };

    let from_body = if let Some(disc) = disc {
        let tag = disc.extract_tag(bitsize as usize);
        quote! {
            let raw = number.value();
            let tag = #tag;
            match tag {
                #( #from_int_match_arms )*
                #catch_all_arm
            }
        }
    } else {
        quote! {
            match number.value() {
                #( #from_int_match_arms )*
                #catch_all_arm
            }
        }
    };

    let assumes = assumes.into_iter().unique_by(TokenStream::to_string);

    quote! {
        impl #const_ ::core::convert::From<#arb_int> for #enum_type {
            fn from(number: #arb_int) -> Self {
                #( #assumes )*
                #from_body
            }
        }
        #from_enum_impl
    }
}

/// a type is considered "filled" if it implements `Bitsized` with `BITS == N`,
/// and additionally is allowed to have any unsigned value from `0` to `2^N - 1`.
/// such a type can then safely implement `From<uN>`.
/// a filled type automatically implements the trait `Filled` thanks to a blanket impl.
/// the check generated by this function will prevent compilation if `ty` is not `Filled`.
fn generate_filled_check_for(ty: &Type, vec: &mut Vec<TokenStream>) {
    use Type::*;
    match ty {
        Path(_) => {
            let assume = quote! { ::bilge::assume_filled::<#ty>(); };
            vec.push(assume);
        }
        Tuple(tuple) => {
            for elem in &tuple.elems {
                generate_filled_check_for(elem, vec)
            }
        }
        Array(array) => generate_filled_check_for(&array.elem, vec),
        _ => unreachable(()),
    }
}

fn generate_struct(arb_int: TokenStream, struct_type: &Ident, fields: &Fields) -> TokenStream {
    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let mut assumes = Vec::new();
    for field in fields {
        generate_filled_check_for(&field.ty, &mut assumes)
    }

    // a single check per type is enough, so the checks can be deduped
    let assumes = assumes.into_iter().unique_by(TokenStream::to_string);

    quote! {
        impl #const_ ::core::convert::From<#arb_int> for #struct_type {
            fn from(value: #arb_int) -> Self {
                #( #assumes )*
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

fn generate_common(expanded: TokenStream) -> TokenStream {
    quote! {
        #expanded
    }
}

fn validate_enum_variants(
    variants: Iter<Variant>, fallback: Option<&Fallback>, disc: Option<&discriminant_at::DiscriminantAt>,
) -> manyhow::Result<()> {
    for variant in variants {
        if let Some(fallback) = &fallback {
            if fallback.is_fallback_variant(&variant.ident) {
                continue;
            }
        }

        if disc.is_some() {
            discriminant_at::variant_payload_ty(variant)?;
            continue;
        }

        if !matches!(variant.fields, Fields::Unit) {
            let help_message = if fallback.is_some() {
                "change this variant to a unit"
            } else {
                "add a fallback variant or change this variant to a unit"
            };
            bail!(variant, "FromBits only supports unit variants for variants without `#[fallback]`"; help = "{}", help_message);
        }
    }
    Ok(())
}
