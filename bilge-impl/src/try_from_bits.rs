use manyhow::bail;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Type, Variant, punctuated::Iter};

use crate::shared::discriminant::EnumDiscriminant;
use crate::shared::{
    self, BitSize, discriminant, discriminant_assigner::DiscriminantAssigner, discriminant_at, enum_fills_bitsize, fallback::Fallback,
    is_always_filled, parse_enum_discriminant, place_struct_fields, unreachable,
};

pub(super) fn try_from_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, internal_bitsize, ..) = analyze(&derive_input)?;
    match derive_data {
        Data::Struct(data) => Ok(codegen_struct(arb_int, name, &data.fields, internal_bitsize)),
        Data::Enum(enum_data) => match parse_enum_discriminant(&derive_input.attrs)? {
            Some(EnumDiscriminant::Type(disc)) => Ok(generate_external_enum(enum_data.variants.iter(), name, &arb_int, &disc)?),
            tag => {
                let disc = match tag {
                    Some(EnumDiscriminant::At(disc)) => {
                        disc.validate(internal_bitsize as usize)?;
                        Some(disc)
                    }
                    _ => None,
                };
                let match_arms = analyze_enum(enum_data.variants.iter(), name, internal_bitsize, &arb_int, disc.as_ref())?;
                Ok(codegen_enum(arb_int, name, match_arms, disc.as_ref(), internal_bitsize))
            }
        },
        _ => unreachable(()),
    }
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> manyhow::Result<(&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>)> {
    shared::analyze_derive(derive_input, true)
}

fn analyze_enum(
    variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize, arb_int: &TokenStream, disc: Option<&discriminant_at::DiscriminantAt>,
) -> manyhow::Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    validate_enum_variants(variants.clone(), disc)?;

    let fill_width = disc.map(|d| d.width as u8).unwrap_or(internal_bitsize);
    let _ = enum_fills_bitsize(fill_width, variants.len())?;

    let mut assigner = DiscriminantAssigner::new(fill_width);

    variants
        .map(|variant| -> manyhow::Result<(TokenStream, TokenStream)> {
            let variant_name = &variant.ident;
            let variant_value = assigner.assign_unsuffixed(variant)?;

            if let Some(disc) = disc {
                let payload_ty = discriminant_at::variant_payload_ty(variant)?;
                let from_int_match_arm =
                    discriminant_at::payload_from_int_arm(disc, internal_bitsize as usize, variant_name, payload_ty, &variant_value, true);
                let to_int_match_arm =
                    discriminant_at::payload_to_int_arm(disc, internal_bitsize as usize, name, variant_name, payload_ty, &variant_value, arb_int);
                Ok((from_int_match_arm, to_int_match_arm))
            } else {
                let from_int_match_arm = quote! {
                    #variant_value => Ok(Self::#variant_name),
                };
                let to_int_match_arm = shared::to_int_match_arm(name, variant_name, arb_int, variant_value);
                Ok((from_int_match_arm, to_int_match_arm))
            }
        })
        .collect::<manyhow::Result<Vec<_>>>()
        .map(|arms| arms.into_iter().unzip())
}

fn generate_external_enum(
    variants: Iter<Variant>, name: &Ident, arb_int: &TokenStream, disc: &discriminant::Discriminant,
) -> manyhow::Result<TokenStream> {
    if let Some(width) = disc.known_width() {
        let _ = enum_fills_bitsize(width, variants.clone().count())?;
    }

    let tag_ty = &disc.ty;
    let mut assigner = DiscriminantAssigner::new(disc.assigner_width());
    let mut from_arms = Vec::new();
    let mut to_arms = Vec::new();
    for variant in variants {
        let payload_ty = discriminant_at::variant_payload_ty(variant)?;
        let variant_value = assigner.assign_unsuffixed(variant)?;
        from_arms.push(discriminant::from_pair_arm(&variant.ident, payload_ty, &variant_value, true));
        to_arms.push(discriminant::to_pair_arm(
            name,
            &variant.ident,
            payload_ty,
            &variant_value,
            tag_ty,
            arb_int,
        ));
    }

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };
    let try_from = discriminant::generate_try_from_pair(name, tag_ty, arb_int, &from_arms, &const_);
    let to_pair = discriminant::generate_pair_from_enum(name, tag_ty, arb_int, &to_arms, &const_);
    Ok(quote! {
        #try_from
        #to_pair
    })
}

fn codegen_enum(
    arb_int: TokenStream, enum_type: &Ident, match_arms: (Vec<TokenStream>, Vec<TokenStream>), disc: Option<&discriminant_at::DiscriminantAt>,
    bitsize: BitSize,
) -> TokenStream {
    let (from_int_match_arms, to_int_match_arms) = match_arms;

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let from_enum_impl = shared::generate_from_enum_impl(&arb_int, enum_type, to_int_match_arms, &const_);

    let try_body = if let Some(disc) = disc {
        let tag = disc.extract_tag(bitsize as usize);
        let tag_bitsize = disc.width as u8;
        let tag_start = disc.start;
        quote! {
            let raw = number.value();
            let tag = #tag;
            match tag {
                #( #from_int_match_arms )*
                _ => Err(::bilge::give_me_error(stringify!(#enum_type), tag as u128, #tag_bitsize).at_offset(#tag_start)),
            }
        }
    } else {
        quote! {
            match number.value() {
                #( #from_int_match_arms )*
                i => Err(::bilge::give_me_error(stringify!(#enum_type), i as u128, #bitsize)),
            }
        }
    };

    quote! {
        impl #const_ ::core::convert::TryFrom<#arb_int> for #enum_type {
            type Error = ::bilge::BitsError;

            fn try_from(number: #arb_int) -> ::core::result::Result<Self, Self::Error> {
                #try_body
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

fn codegen_struct(arb_int: TokenStream, struct_type: &Ident, fields: &Fields, declared_bitsize: BitSize) -> TokenStream {
    let layout = place_struct_fields(fields, declared_bitsize as usize).unwrap_or_else(|_| unreachable(()));
    let field_checks: Vec<TokenStream> = fields
        .iter()
        .zip(layout.fields.iter())
        .enumerate()
        .filter_map(|(i, (field, place))| {
            let ty = &field.ty;
            // primitives, and arrays/tuples of them, always convert
            if is_always_filled(ty) {
                return None;
            }
            let offset = &place.offset;
            let check = generate_field_check(ty);
            let field_name = field.ident.clone().unwrap_or_else(|| format_ident!("val_{i}"));
            Some(quote! {
                cursor = value.value();
                cursor >>= #offset;
                match { #check } {
                    Ok(()) => {}
                    Err(e) => return Err(e.in_field(stringify!(#field_name), #offset)),
                }
            })
        })
        .collect();

    let cursor_setup = if field_checks.is_empty() {
        quote!()
    } else {
        quote! {
            type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
            type BaseIntOf<T> = <ArbIntOf<T> as Integer>::UnderlyingType;

            // cursor starts at value's first field
            let mut cursor = value.value();
            #(#field_checks)*
        }
    };

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    quote! {
        impl #const_ ::core::convert::TryFrom<#arb_int> for #struct_type {
            type Error = ::bilge::BitsError;

            // validates all values, which means enums, even in inner structs (TODO: and reserved fields?)
            fn try_from(value: #arb_int) -> ::core::result::Result<Self, Self::Error> {
                #cursor_setup
                Ok(Self { value })
            }
        }

        impl #const_ ::core::convert::From<#struct_type> for #arb_int {
            fn from(struct_value: #struct_type) -> Self {
                struct_value.value
            }
        }
    }
}

fn validate_enum_variants(variants: Iter<Variant>, disc: Option<&discriminant_at::DiscriminantAt>) -> manyhow::Result<()> {
    for variant in variants {
        if disc.is_some() {
            discriminant_at::variant_payload_ty(variant)?;
            continue;
        }
        if !matches!(variant.fields, Fields::Unit) {
            bail!(variant, "TryFromBits only supports unit variants in enums"; help = "change this variant to a unit");
        }
    }
    Ok(())
}
