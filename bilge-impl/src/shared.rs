pub mod discriminant_assigner;
pub mod fallback;
pub mod util;

use fallback::{fallback_variant, Fallback};
use proc_macro2::{Ident, Literal, TokenStream};
use proc_macro_error::{abort, abort_call_site};
use quote::{format_ident, quote};
use syn::{
    parse::{ParseStream, Parser},
    spanned::Spanned,
    Attribute, DeriveInput, Error, LitInt, Meta, Result, Token, Type,
};
use util::PathExt;

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

// allow since we want `if try_from` blocks to stand out
#[allow(clippy::collapsible_if)]
pub(crate) fn analyze_derive(derive_input: &DeriveInput, try_from: bool) -> (&syn::Data, TokenStream, &Ident, BitSize, Option<Fallback>) {
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

    // parsing the #[bitsize_internal(num)] attribute macro
    let args = attrs
        .iter()
        .find_map(bitsize_internal_arg)
        .unwrap_or_else(|| abort_call_site!("add #[bitsize] attribute above your derive attribute"));
    let (bitsize, arb_int, _) = bitsize_and_arbitrary_int_from(args);

    let fallback = fallback_variant(data, bitsize);
    if fallback.is_some() && try_from {
        abort_call_site!("fallback is not allowed with `TryFromBits`"; help = "use `#[derive(FromBits)]` or remove this `#[fallback]`")
    }

    (data, arb_int, ident, bitsize, fallback)
}

// If we want to support bitsize(u4) besides bitsize(4), do that here.
pub fn bitsize_and_arbitrary_int_from(bitsize_arg: TokenStream) -> (BitSize, TokenStream, FieldLayout) {
    let parser = |input: syn::parse::ParseStream| -> Result<(LitInt, Option<Ident>)> {
        let bitsize: LitInt = input.parse()?;
        let layout = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let layout: Ident = input.parse()?;
            Some(layout)
        } else {
            None
        };
        Ok((bitsize, layout))
    };

    let (bitsize, layout) = parser
        .parse2(bitsize_arg.clone())
        .unwrap_or_else(|_| abort!(bitsize_arg, "invalid arguments"; help = "arguments format: (<bit size> [, auto | manual])"));

    // without postfix
    let bitsize = bitsize
        .base10_parse()
        .ok()
        .filter(|&n| n != 0 && n <= MAX_STRUCT_BIT_SIZE)
        .unwrap_or_else(
            || abort!(bitsize_arg, "bit size argument is not a valid number"; help = "currently, numbers from 1 to {} are allowed", MAX_STRUCT_BIT_SIZE),
        );

    let layout = match layout {
        Some(layout) => FieldLayout::parse(layout)
            .unwrap_or_else(|_| abort!(bitsize_arg, "invalid layout (optional) argument"; help = "two options allowed: manual or auto")),
        None => FieldLayout::Auto,
    };
    let arb_int = syn::parse_str(&format!("u{bitsize}")).unwrap_or_else(unreachable);
    (bitsize, arb_int, layout)
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
            quote!(<#ty as Bitsized>::BITS)
        }
        _ => unreachable(()),
    }
}

pub(crate) fn generate_from_enum_impl(
    arb_int: &TokenStream, enum_type: &Ident, to_int_match_arms: Vec<TokenStream>, const_: &TokenStream,
) -> TokenStream {
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
/// Currently, this is exactly the set of types we can extract a bitsize out of, just by looking at their ident: `uN` and `bool`.
pub fn is_always_filled(ty: &Type) -> bool {
    last_ident_of_path(ty).and_then(bitsize_from_type_ident).is_some()
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
pub fn enum_fills_bitsize(bitsize: u8, variants_count: usize) -> bool {
    let max_variants_count = 1u128 << bitsize;
    if variants_count as u128 > max_variants_count {
        abort_call_site!("enum overflows its bitsize"; help = "there should only be at most {} variants defined", max_variants_count);
    }
    variants_count as u128 == max_variants_count
}

#[inline]
pub fn unreachable<T, U>(_: T) -> U {
    unreachable!("should have already been validated")
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

/// attempts to extract the bitsize from an ident equal to `uN` or `bool`.
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

/// Specify field layout:
/// 1. Auto: All fields are adjacent to each other, without gaps or overlaps.
/// 2. Manual: Manually specify bit range of each field, allows gaps and overlaps.
#[derive(Debug, Clone, Copy)]
pub enum FieldLayout {
    Auto,
    Manual,
}
impl FieldLayout {
    fn parse(arg: Ident) -> Result<Self> {
        match arg.to_string().as_str() {
            "auto" => Ok(FieldLayout::Auto),
            "manual" => Ok(FieldLayout::Manual),
            _ => Err(Error::new_spanned(arg, "invalid layout argument, expect: (manual | auto)")),
        }
    }
    pub fn ident(&self) -> Ident {
        match self {
            FieldLayout::Auto => format_ident!("auto"),
            FieldLayout::Manual => format_ident!("manual"),
        }
    }
}

/// Bit range data including bit range, access mode of field.
pub struct BitRange {
    /// Index offset of start bit
    pub start_bit: usize,
    pub bit_size: usize,
    pub access: Access,
}
pub enum Access {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}
impl Access {
    fn parse(i: Ident) -> Result<Access> {
        match i.to_string().as_str() {
            "r" => Ok(Access::ReadOnly),
            "w" => Ok(Access::WriteOnly),
            "rw" => Ok(Access::ReadWrite),
            _ => Err(Error::new(i.span(), "invalid access mode, expect: (r | w | rw)")),
        }
    }
}
pub enum BitRangeAttr<'a> {
    /// 1. Single bit: #[bit(<index>)]
    BitAttr(&'a Attribute),
    /// 2. Multiple bits: #[bits(<start>..[=]<end>)]]
    BitsAttr(&'a Attribute),
}
impl<'a> BitRangeAttr<'a> {
    pub fn parse(&self) -> Result<BitRange> {
        match self {
            BitRangeAttr::BitAttr(attr) => attr
                .parse_args_with(Self::parse_bit_attr)
                .map_err(|e| syn::Error::new(attr.span(), format!("{}, should be like: #[bit(7, rw)]", e))),
            BitRangeAttr::BitsAttr(attr) => attr
                .parse_args_with(Self::parse_bits_attr)
                .map_err(|e| syn::Error::new(attr.span(), format!("{}, should be like: #[bits(5..=10, r)]", e))),
        }
    }

    pub fn raw(&self) -> &Attribute {
        match self {
            BitRangeAttr::BitAttr(attr) => attr,
            BitRangeAttr::BitsAttr(attr) => attr,
        }
    }

    fn parse_bit_attr(input: ParseStream) -> Result<BitRange> {
        let bit_index = input.parse::<LitInt>()?.base10_parse::<usize>()?;
        let access = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let access: Ident = input.parse()?;
            Access::parse(access)?
        } else {
            Access::ReadWrite
        };
        Ok(BitRange {
            start_bit: bit_index,
            bit_size: 1,
            access,
        })
    }

    fn parse_bits_attr(input: ParseStream) -> Result<BitRange> {
        let span = input.span();
        let start_bit = input.parse::<LitInt>()?.base10_parse::<usize>()?;
        let _ = input.parse::<Token![..]>()?;
        let inclusive: Option<Token![=]> = input.parse()?;
        let end_bit = input.parse::<LitInt>()?.base10_parse::<usize>()?;
        let bit_size = if inclusive.is_some() {
            if start_bit > end_bit {
                return Err(Error::new(span, "expected start <= end in #[bits(start..=end)]"));
            }
            end_bit + 1 - start_bit
        } else {
            if start_bit >= end_bit {
                return Err(Error::new(span, "expected start < end in #[btis(start..end)]"));
            }
            end_bit - start_bit
        };
        let access = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let access: Ident = input.parse()?;
            Access::parse(access)?
        } else {
            Access::ReadWrite
        };
        Ok(BitRange { start_bit, bit_size, access })
    }
}
// Find #[bit] or [#bits].
pub fn find_bit_range_attr(attrs: &[Attribute]) -> Option<BitRangeAttr> {
    for attr in attrs {
        if attr.path().is_ident("bit") {
            return Some(BitRangeAttr::BitAttr(attr));
        } else if attr.path().is_ident("bits") {
            return Some(BitRangeAttr::BitsAttr(attr));
        }
    }
    None
}
