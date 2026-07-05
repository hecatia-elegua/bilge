use manyhow::manyhow;
use proc_macro2::TokenStream;

mod bitsize;
mod bitsize_internal;
mod debug_bits;
mod default_bits;
mod fmt_bits;
mod from_bits;
#[cfg(feature = "schemars")]
#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
mod schemars_bits;
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod serde_bits;
mod try_from_bits;

mod shared;

/// Defines the bitsize of a struct or an enum.
///
/// e.g. `#[bitsize(4)]` represents the item as a u4, which is UInt<u8, 4> underneath.
/// The size of structs is currently limited to 128 bits.
/// The size of enums is limited to 64 bits.
/// Please open an issue if you have a usecase for bigger bitfields.
#[manyhow]
#[proc_macro_attribute]
pub fn bitsize(args: TokenStream, item: TokenStream) -> manyhow::Result {
    bitsize::bitsize(args, item)
}

/// This is internally used, not to be used by anything besides `bitsize`.
/// No guarantees are given.
#[manyhow]
#[proc_macro_attribute]
pub fn bitsize_internal(args: TokenStream, item: TokenStream) -> manyhow::Result {
    bitsize_internal::bitsize_internal(args, item)
}

/// Generate an `impl TryFrom<uN>` for unfilled bitfields.
///
/// This should be used when your enum or enums nested in
/// a struct don't fill their given `bitsize`.
#[manyhow]
#[proc_macro_derive(TryFromBits, attributes(bitsize_internal, fallback))]
pub fn derive_try_from_bits(item: TokenStream) -> manyhow::Result {
    try_from_bits::try_from_bits(item)
}

/// Generate an `impl From<uN>` for filled bitfields.
///
/// This should be used when your enum or enums nested in
/// a struct fill their given `bitsize` or if you're not
/// using enums.
#[manyhow]
#[proc_macro_derive(FromBits, attributes(bitsize_internal, fallback))]
pub fn derive_from_bits(item: TokenStream) -> manyhow::Result {
    from_bits::from_bits(item)
}

/// Generate an `impl core::fmt::Debug` for bitfield structs.
///
/// Please use normal #[derive(Debug)] for enums.
#[manyhow]
#[proc_macro_derive(DebugBits, attributes(bitsize_internal))]
pub fn debug_bits(item: TokenStream) -> manyhow::Result {
    debug_bits::debug_bits(item)
}

/// Generate an `impl core::fmt::Binary` for bitfields.
#[manyhow]
#[proc_macro_derive(BinaryBits)]
pub fn derive_binary_bits(item: TokenStream) -> manyhow::Result {
    fmt_bits::binary(item)
}

/// Generate an `impl core::default::Default` for bitfield structs.
#[manyhow]
#[proc_macro_derive(DefaultBits)]
pub fn derive_default_bits(item: TokenStream) -> manyhow::Result {
    default_bits::default_bits(item)
}

/// Generate an `impl schemars::JsonSchema` for bitfield structs.
///
/// Please use normal #[derive(JsonSchema)] for enums.
#[cfg(feature = "schemars")]
#[manyhow]
#[proc_macro_derive(JsonSchemaBits, attributes(bitsize_internal))]
pub fn json_schema_bits(item: TokenStream) -> manyhow::Result {
    schemars_bits::json_schema_bits(item)
}

/// Generate an `impl serde::Serialize` for bitfield structs.
///
/// Please use normal #[derive(Serialize)] for enums.
#[cfg(feature = "serde")]
#[manyhow]
#[proc_macro_derive(SerializeBits, attributes(bitsize_internal))]
pub fn serialize_bits(item: TokenStream) -> manyhow::Result {
    serde_bits::serialize_bits(item)
}

/// Generate an `impl serde::Deserialize` for bitfield structs.
///
/// Please use normal #[derive(Deserialize)] for enums.
#[cfg(feature = "serde")]
#[manyhow]
#[proc_macro_derive(DeserializeBits, attributes(bitsize_internal))]
pub fn deserialize_bits(item: TokenStream) -> manyhow::Result {
    serde_bits::deserialize_bits(item)
}
