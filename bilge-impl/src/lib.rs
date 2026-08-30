use manyhow::manyhow;
use proc_macro2::TokenStream;

mod bitsize;
mod bitsize_internal;
mod builder_bits;
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
///
/// After the size, structs may take extra options:
/// - `hide_value`: place the generated struct in a private module so the raw
///   `value` field cannot be read or written. To write the whole value, use `From`/`TryFrom` instead, which is more type-safe.
///   Relative visibility (`pub(super)`, `pub(self)`, inherited) is shifted one `super`
///   so it works like you wrote it.
/// - `new = <vis>`: visibility of `new` (private by default, like other Rust items).
/// Examples: `new = pub`, `new = pub(crate)`.
///
/// Struct fields may use `#[at(n)]` or `#[at(n..=m)]` to place a field at backing-integer
/// bit `n` (0 = LSB). Width comes from the field type; a range is checked against that width.
/// Fields without `#[at]` fill sequentially from the end of the previous
/// field. Skipped bits are implicit padding. Overlap or reordering is a compile error.
///
/// Enums may use `#[discriminant_at(n)]` or `#[discriminant_at(n..=m)]` when the tag sits in
/// the same integer, and `#[discriminant(TagType)]` when the tag is a separate value.
/// Variants look like `Variant(Payload) = tag`. Use `TryFromBits` when not every tag is used.
///
/// ```ignore
/// #[bitsize(8, hide_value, new = pub(crate))]
/// struct Register { ... }
/// ```
#[manyhow]
#[proc_macro_attribute]
pub fn bitsize(args: TokenStream, item: TokenStream) -> manyhow::Result {
    bitsize::bitsize(args, item)
}

/// This is internally used, not to be used by anything besides `bitsize`.
/// No guarantees are given.
#[doc(hidden)]
#[manyhow]
#[proc_macro_attribute]
pub fn bitsize_internal(args: TokenStream, item: TokenStream) -> manyhow::Result {
    bitsize_internal::bitsize_internal(args, item)
}

/// Generate a compile-time builder (`typed-builder` style: each field exactly once).
///
/// Required fields have no `#[default]` and must be set before `build`.
/// `#[default(expr)]` makes a field optional (same as in `DefaultBits`).
/// `reserved` / `padding` are omitted, same as in `new`.
#[manyhow]
#[proc_macro_derive(BuilderBits, attributes(bitsize_internal, at, default))]
pub fn derive_builder_bits(item: TokenStream) -> manyhow::Result {
    builder_bits::builder_bits(item)
}

/// Generate an `impl TryFrom<uN>` for unfilled bitfields.
///
/// This should be used when your enum or enums nested in
/// a struct don't fill their given `bitsize`.
#[manyhow]
#[proc_macro_derive(TryFromBits, attributes(bitsize_internal, fallback, at, discriminant_at, discriminant))]
pub fn derive_try_from_bits(item: TokenStream) -> manyhow::Result {
    try_from_bits::try_from_bits(item)
}

/// Generate an `impl From<uN>` for filled bitfields.
///
/// This should be used when your enum or enums nested in
/// a struct fill their given `bitsize` or if you're not
/// using enums.
#[manyhow]
#[proc_macro_derive(FromBits, attributes(bitsize_internal, fallback, at, discriminant_at, discriminant))]
pub fn derive_from_bits(item: TokenStream) -> manyhow::Result {
    from_bits::from_bits(item)
}

/// Generate an `impl core::fmt::Debug` for bitfield structs.
///
/// Please use normal #[derive(Debug)] for enums.
#[manyhow]
#[proc_macro_derive(DebugBits, attributes(bitsize_internal, at))]
pub fn debug_bits(item: TokenStream) -> manyhow::Result {
    debug_bits::debug_bits(item)
}

/// Generate an `impl core::fmt::Binary` for bitfields.
#[manyhow]
#[proc_macro_derive(BinaryBits, attributes(bitsize_internal, fallback, at, discriminant_at, discriminant))]
pub fn derive_binary_bits(item: TokenStream) -> manyhow::Result {
    fmt_bits::binary(item)
}

/// Generate an `impl core::default::Default` for bitfield structs.
///
/// Each field uses `T::default()` unless it has `#[default(expr)]`, same as in `BuilderBits`.
#[manyhow]
#[proc_macro_derive(DefaultBits, attributes(bitsize_internal, at, default))]
pub fn derive_default_bits(item: TokenStream) -> manyhow::Result {
    default_bits::default_bits(item)
}

/// Generate an `impl schemars::JsonSchema` for bitfield structs.
///
/// Please use normal #[derive(JsonSchema)] for enums.
#[cfg(feature = "schemars")]
#[manyhow]
#[proc_macro_derive(JsonSchemaBits, attributes(bitsize_internal, at))]
pub fn json_schema_bits(item: TokenStream) -> manyhow::Result {
    schemars_bits::json_schema_bits(item)
}

/// Generate an `impl serde::Serialize` for bitfield structs.
///
/// Please use normal #[derive(Serialize)] for enums.
#[cfg(feature = "serde")]
#[manyhow]
#[proc_macro_derive(SerializeBits, attributes(bitsize_internal, at))]
pub fn serialize_bits(item: TokenStream) -> manyhow::Result {
    serde_bits::serialize_bits(item)
}

/// Generate an `impl serde::Deserialize` for bitfield structs.
///
/// Please use normal #[derive(Deserialize)] for enums.
#[cfg(feature = "serde")]
#[manyhow]
#[proc_macro_derive(DeserializeBits, attributes(bitsize_internal, at))]
pub fn deserialize_bits(item: TokenStream) -> manyhow::Result {
    serde_bits::deserialize_bits(item)
}
