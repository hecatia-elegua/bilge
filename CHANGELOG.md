# Changelog

## [Unreleased]

### Added
- `#[at(n)]` / `#[at(n..=m)]` on struct fields to place a field at backing-integer bit `n` (0 = LSB). Width comes from the field type; a range is checked against that width. Fields without `#[at]` continue after the previous one. Skipped bits are implicit padding. Overlap or going backwards is a compile error. `#[bitsize(N)]` is still the total width.
- `#[discriminant_at(n)]` / `#[discriminant_at(n..=m)]` on enums when the tag sits in the same value. Variants look like `Variant(Payload) = tag`. Use `TryFromBits` when not every tag is used. `#[fallback] Raw(uN)` (full enum width) keeps unknown tags as the whole word.
- `#[discriminant(SomeTagType)]` on enums when the tag is a separate value. `From`/`TryFrom` will then use `(tag, bits)` as the raw values. Tag types are limited to 64 bits, same as enums.
- `toggle_*` for `bool` fields and `toggle_*_at` for `[bool; N]`
- `BuilderBits` named typestate builder (`SomeType::builder().field(v).build()`). Required fields must be set exactly once; `#[default(expr)]` makes a field optional. This does not require `DefaultBits`.
- `#[default(expr)]` on struct fields is also honored by `DefaultBits`
- `DeserializeBits` treats `#[default(expr)]` like `#[serde(default = "...")]` (the field may be omitted). `#[at]` holes are omitted like `reserved` / `padding` and reconstruct as 0 via `new`.

### Changed
- `BitsError` now names the innermost failing type, a short field path (`wrapper.inner.bar`, tuple positions as `0`), the invalid bit pattern, and that field's bit range in the value passed to `try_from`. We only keep one level of array depth, so prefer the bit-range for accurate handling.
- `Bitsized` now has `as_int(&self)` so by-ref conversions do not need `Clone`. Manual `Bitsized` impls need this method.

### Fixed
- `BinaryBits` (and owned `From` for tagged payload enums) no longer requires payload types to implement `Clone`
- `BinaryBits` can be used with `#[non_exhaustive]` + `TryFromBits` enums
- Only field names that start with `reserved_` / `padding_` are treated as reserved (not names that merely contain those strings)
- Generated code no longer requires `bilge::prelude` for `Bitsized` / `Integer`

## [0.4.0] - 2026-08-22

### Added
- `schemars` feature and `JsonSchemaBits` derive, added by [widberg](https://github.com/widberg)
- `#[bitsize(N, hide_value)]` to hide the generated `value` field for more type-safety; optional because it is often useful to have full access (besides that you can still use `From`/`TryFrom`). Relative visibility is shifted one `super` up to work like usual.
- `#[bitsize(N, new = <vis>)]` to set constructor visibility (private by default; e.g. `new = pub`, `new = pub(crate)`)
- `BitsError` now implements `Clone`, `Copy`, `Eq`, `Hash`, and `core::error::Error`

### Changed
- Generated `new` is private by default (BREAKING if you called it from another module -> add `new = pub`)
- Edition 2024; MSRV is 1.85
- Internal helpers are `#[doc(hidden)]` (these should not be used), `Bitsized` is now part of the API
- Generated bitstructs are `#[repr(transparent)]` over the backing integer (needed for zerocopy 0.8 `FromBytes`)
- Dependencies: `arbitrary-int` 2.2, `itertools` 0.15
- Proc-macro errors now use [manyhow](https://crates.io/crates/manyhow), added by [widberg](https://github.com/widberg)

### Fixed
- Bitfields compile when `defmt` shadows formatting macros, by [Willa Hughes](https://github.com/WillaWillNot)

