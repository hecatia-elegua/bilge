# Changelog

## [Unreleased]

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

