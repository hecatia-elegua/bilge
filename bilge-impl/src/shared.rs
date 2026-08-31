pub mod at;
pub mod bitsize_args;
pub mod discriminant;
pub mod discriminant_assigner;
pub mod discriminant_at;
pub mod fallback;
pub mod util;

pub use at::{attrs_without_at, binary_segments, is_at_attribute, place_struct_fields};
pub use bitsize_args::{BitsizeArgs, internal_attr_options, parse_bitsize_args};
pub use discriminant::{is_discriminant_attribute, parse_enum_discriminant};
pub use discriminant_at::{is_discriminant_at_attribute, parse_discriminant_at};
use fallback::{Fallback, fallback_variant};
use manyhow::{bail, ensure};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, Expr, Field, Meta, Type};
use util::PathExt;

/// Bring `Bitsized` and `Integer` methods into generated bodies without requiring `bilge::prelude`.
pub fn import_traits() -> TokenStream {
    quote! {
        #[allow(unused_imports)]
        use ::bilge::Bitsized as _;
        #[allow(unused_imports)]
        use ::bilge::arbitrary_int::traits::Integer as _;
    }
}

/// As arbitrary_int is limited to basic rust primitives, the maximum is u128.
/// Is there a true usecase for bitfields above this size?
/// This would also be change-worthy when rust starts supporting LLVM's arbitrary integers.
pub const MAX_STRUCT_BIT_SIZE: BitSize = 128;
/// As `#[repr(u128)]` is unstable and currently no real usecase for higher sizes exists, the maximum is u64.
pub const MAX_ENUM_BIT_SIZE: BitSize = 64;
pub type BitSize = u8;

pub(crate) fn parse_derive(item: TokenStream) -> DeriveInput {
    syn::parse2(item).unwrap_or_else(unreachable)
}

#[derive(Clone, Copy)]
pub(crate) enum DeriveKind {
    FromBits,
    TryFromBits,
    /// Formatting and other derives that only need the bitsize, not conversion policy.
    Other,
}

pub(crate) fn analyze_derive(
    derive_input: &DeriveInput, kind: DeriveKind,
) -> manyhow::Result<(&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>)> {
    let DeriveInput {
        attrs,
        ident,
        // generics,
        data,
        ..
    } = derive_input;

    match kind {
        DeriveKind::FromBits => {
            if attrs.iter().any(is_non_exhaustive_attribute) {
                bail!("Item can't be FromBits and non_exhaustive"; help = "remove #[non_exhaustive] or derive(FromBits) here")
            }
        }
        DeriveKind::TryFromBits => {
            // currently not allowed, would need some thinking:
            if let syn::Data::Struct(_) = data {
                if attrs.iter().any(is_non_exhaustive_attribute) {
                    bail!("Using #[non_exhaustive] on structs is currently not supported"; help = "open an issue on our repository if needed")
                }
            }
        }
        DeriveKind::Other => {}
    }

    // parsing the #[bitsize_internal(num)] attribute macro
    ensure!(
        let Some(args) = attrs.iter().find_map(bitsize_internal_arg),
        "add #[bitsize] attribute above your derive attribute"
    );
    let (bitsize, arb_int) = bitsize_and_arbitrary_int_from(args)?;

    let fallback = fallback_variant(data, bitsize)?;
    if fallback.is_some() && matches!(kind, DeriveKind::TryFromBits) {
        bail!("fallback is not allowed with `TryFromBits`"; help = "use `#[derive(FromBits)]` or remove this `#[fallback]`")
    }

    Ok((data, arb_int, ident, bitsize, fallback))
}

// If we want to support bitsize(u4) besides bitsize(4), do that here.
pub fn bitsize_and_arbitrary_int_from(bitsize_arg: TokenStream) -> manyhow::Result<(BitSize, TokenStream)> {
    let args = parse_bitsize_args(bitsize_arg)?;
    Ok((args.bitsize, args.arb_int))
}

pub fn generate_type_bitsize(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            tuple
                .elems
                .iter()
                .map(generate_type_bitsize)
                .reduce(|acc, next| quote!((#acc + #next)))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        }
        Array(array) => {
            let elem_bitsize = generate_type_bitsize(&array.elem);
            let len_expr = &array.len;
            quote!((#elem_bitsize * #len_expr))
        }
        Path(_) => {
            quote!(<#ty as ::bilge::Bitsized>::BITS)
        }
        _ => unreachable(()),
    }
}

pub(crate) fn generate_from_enum_impl(
    arb_int: &TokenStream, enum_type: &Ident, to_int_match_arms: Vec<TokenStream>, const_: &TokenStream,
) -> TokenStream {
    let import = import_traits();
    quote! {
        impl #const_ ::core::convert::From<#enum_type> for #arb_int {
            fn from(enum_value: #enum_type) -> Self {
                #import
                match enum_value {
                    #( #to_int_match_arms )*
                }
            }
        }
    }
}

/// Filters fields which are always `FILLED`, meaning all bit-patterns are possible,
/// meaning they are (should be) From<uN>, not TryFrom<uN>.
///
/// Currently, this is exactly the set of types we can extract  a bitsize out of, just by looking at their ident: `uN` and  `bool`.
/// Arrays and tuples of those of course too.
/// Nested `FromBits` structs are not detected, since we can't see `Filled` from the ident.
pub fn is_always_filled(ty: &Type) -> bool {
    match ty {
        Type::Tuple(tuple) => tuple.elems.iter().all(is_always_filled),
        Type::Array(array) => is_always_filled(&array.elem),
        Type::Path(_) => last_ident_of_path(ty).and_then(bitsize_from_type_ident).is_some(),
        _ => false,
    }
}

pub fn last_ident_of_path(ty: &Type) -> Option<&Ident> {
    if let Type::Path(type_path) = ty {
        // the type may have a qualified path, so I don't think we can use `get_ident()` here
        let last_segment = type_path.path.segments.last()?;
        Some(&last_segment.ident)
    } else {
        None
    }
}

/// in enums, internal_bitsize <= 64; u64::MAX + 1 = u128
/// therefore the bitshift would not overflow.
pub fn enum_fills_bitsize(bitsize: u8, variants_count: usize) -> manyhow::Result<bool> {
    let max_variants_count = 1u128 << bitsize;
    if variants_count as u128 > max_variants_count {
        bail!("enum overflows its bitsize"; help = "there should only be at most {} variants defined", max_variants_count);
    }
    Ok(variants_count as u128 == max_variants_count)
}

#[inline]
pub fn unreachable<T, U>(_: T) -> U {
    ::core::unreachable!("should have already been validated")
}

pub fn is_attribute(attr: &Attribute, name: &str) -> bool {
    if let Meta::Path(path) = &attr.meta { path.is_ident(name) } else { false }
}

pub fn is_default_attribute(attr: &Attribute) -> bool {
    attr.path().is_ident("default")
}

/// `#[default(expr)]` on a struct field.
pub fn parse_field_default(field: &Field) -> manyhow::Result<Option<Expr>> {
    let mut found = None;
    for attr in &field.attrs {
        if !is_default_attribute(attr) {
            continue;
        }
        ensure!(found.is_none(), attr, "duplicate `#[default]`");
        found = Some(attr.parse_args()?);
    }
    Ok(found)
}

pub fn reject_default_on_reserved(field: &Field, name: &str, has_default: bool, for_default_bits: bool) -> manyhow::Result<()> {
    if is_reserved_or_padding(name) && has_default {
        if for_default_bits {
            bail!(
                field,
                "`#[default]` is not supported on reserved/padding fields";
                help = "construction leaves those bits 0, From/TryFrom keep the raw value"
            );
        } else {
            bail!(
                field,
                "`#[default]` is not supported on reserved/padding fields";
                help = "they are omitted from `new` and the builder"
            );
        }
    }
    Ok(())
}

pub fn is_reserved_or_padding(name: &str) -> bool {
    name.starts_with("reserved_") || name.starts_with("padding_")
}

fn is_non_exhaustive_attribute(attr: &Attribute) -> bool {
    is_attribute(attr, "non_exhaustive")
}

pub(crate) fn is_fallback_attribute(attr: &Attribute) -> bool {
    is_attribute(attr, "fallback")
}

/// attempts to extract the bitsize from an ident equal to `uN`, `iN` or `bool`.
/// should return `Result` instead of `Option`, if we decide to add more descriptive error handling.
pub fn bitsize_from_type_ident(type_name: &Ident) -> Option<BitSize> {
    let type_name = type_name.to_string();

    if type_name == "bool" {
        Some(1)
    } else if let Some(suffix) = type_name.strip_prefix('u') {
        // characters which may appear in this suffix are digits, letters and underscores.
        // parse() will reject letters and underscores, so this should be correct.
        let bitsize = suffix.parse().ok();

        // the namespace contains u2 up to u{MAX_STRUCT_BIT_SIZE}. can't make assumptions about larger values
        bitsize.filter(|&n| n <= MAX_STRUCT_BIT_SIZE)
    } else if let Some(suffix) = type_name.strip_prefix('i') {
        let bitsize = suffix.parse().ok();
        bitsize.filter(|&n| n <= MAX_STRUCT_BIT_SIZE)
    } else {
        None
    }
}

pub fn to_int_match_arm(enum_name: &Ident, variant_name: &Ident, arb_int: &TokenStream, variant_value: Literal) -> TokenStream {
    quote! { #enum_name::#variant_name => #arb_int::new(#variant_value), }
}

pub(crate) fn bitsize_internal_arg(attr: &Attribute) -> Option<TokenStream> {
    if let Meta::List(list) = &attr.meta {
        if list.path.matches(&["bilge", "bitsize_internal"]) {
            let arg = list.tokens.to_owned();
            return Some(arg);
        }
    }

    None
}
