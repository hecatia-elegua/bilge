#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![no_std]

use core::fmt;

#[doc(no_inline)]
pub use arbitrary_int;
#[cfg(feature = "schemars")]
#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
pub use bilge_impl::JsonSchemaBits;
#[doc(hidden)]
pub use bilge_impl::bitsize_internal;
pub use bilge_impl::{BinaryBits, BuilderBits, DebugBits, DefaultBits, FromBits, TryFromBits, bitsize};
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub use bilge_impl::{DeserializeBits, SerializeBits};

/// used for `use bilge::prelude::*;`
pub mod prelude {
    #[rustfmt::skip]
    #[doc(no_inline)]
    pub use super::{
        bitsize, Bitsized,
        FromBits, TryFromBits, DebugBits, BinaryBits, DefaultBits, BuilderBits,
        // we control the version, so this should not be a problem
        arbitrary_int::prelude::*,
    };
    #[cfg(feature = "schemars")]
    #[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
    pub use super::JsonSchemaBits;
    #[cfg(feature = "serde")]
    #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
    pub use super::{DeserializeBits, SerializeBits};
}

/// Used by generated code to talk about a bitfield's backing integer.
pub trait Bitsized {
    /// The arbitrary_int type, used internally to 'generically' access its methods.
    type ArbitraryInt;
    /// The number of bits this type uses.
    const BITS: usize;
    /// The maximum value this type can hold.
    const MAX: Self::ArbitraryInt;
    /// The backing integer, from `&self`.
    ///
    /// `From<Self> for Self::ArbitraryInt` takes `Self` by value. This is the
    /// `&self` equivalent, so formatting and other by-ref conversions do not need [`Clone`].
    fn as_int(&self) -> Self::ArbitraryInt;
}

/// Internally used marker trait.
///
/// # Safety
///
/// Avoid implementing this for your types. Implementing this trait could break invariants.
#[doc(hidden)]
pub unsafe trait Filled: Bitsized {}
#[doc(hidden)]
unsafe impl<T> Filled for T where T: Bitsized + From<<T as Bitsized>::ArbitraryInt> {}

/// This is generated to statically validate that a type implements `FromBits`.
#[doc(hidden)]
pub const fn assume_filled<T: Filled>() {}

/// Nested field names reported by [`BitsError`]. Inside-out, so beyond this, outer names are dropped.
const MAX_FIELD_PATH: usize = 8;

/// Error returned by `TryFromBits` conversions.
///
/// Conversion stops at the first invalid field.
/// Nested structs keep the innermost failing type (for example `Bar` in `Nested { inner: Byte { bar: Bar } }`)
/// and the bit pattern that had no matching representation.
///
/// [`Self::bit_start`] / [`Self::bit_end`] locate the field more accurately by bit range.
/// [`Self::field_path`] is extra context (named fields and tuple positions like `0`),
/// truncated to at most [`MAX_FIELD_PATH`] names.
///
/// Fields are private so more context can be added later without breaking pattern matching.
/// Use the accessors below.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitsError {
    type_name: &'static str,
    /// Struct fields and tuple positions from the type you called `try_from` on down to this error
    /// (`["inner", "bar"]`, or `["data", "0"]` for a tuple element).
    field_path: [&'static str; MAX_FIELD_PATH],
    field_path_len: u8,
    /// The pattern that did not match.
    invalid_bits: u128,
    bitsize: u8,
    /// The bit range of the field that did not match in the value passed to `try_from` (0 = LSB).
    bit_start: u8,
}

impl BitsError {
    /// Name of the type that could not be constructed.
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Innermost path segment: a field name, or a tuple index like `"0"`.
    pub const fn field_name(&self) -> Option<&'static str> {
        if self.field_path_len == 0 {
            None
        } else {
            Some(self.field_path[self.field_path_len as usize - 1])
        }
    }

    /// Field names and tuple positions from the `try_from` type down to the failure, e.g.
    /// `["inner", "bar"]` or `["data", "1"]`. At most [`MAX_FIELD_PATH`] names.
    pub const fn field_path(&self) -> &[&'static str] {
        self.field_path.split_at(self.field_path_len as usize).0
    }

    /// The invalid bit pattern.
    pub const fn invalid_bits(&self) -> u128 {
        self.invalid_bits
    }

    /// Width of [`bits`](Self::invalid_bits).
    pub const fn bitsize(&self) -> u8 {
        self.bitsize
    }

    /// First bit of the failing value in the integer passed to `try_from` (0 = LSB).
    pub const fn bit_start(&self) -> u8 {
        self.bit_start
    }

    /// Last bit of the failing value (inclusive).
    pub const fn bit_end(&self) -> u8 {
        self.bit_start.saturating_add(self.bitsize.saturating_sub(1))
    }

    /// Remember the field that produced this error and shift its bit range by that field's offset in the parent integer.
    /// Outer names are prepended so the path reads `wrapper.inner.bar`.
    #[doc(hidden)]
    pub const fn in_field(self, field_name: &'static str, bit_start: usize) -> Self {
        let mut path = self.field_path;
        let mut len = self.field_path_len;
        if len == 0 {
            path[0] = field_name;
            len = 1;
        } else if (len as usize) < MAX_FIELD_PATH {
            let mut i = len as usize;
            while i > 0 {
                path[i] = path[i - 1];
                i -= 1;
            }
            path[0] = field_name;
            len += 1;
        }
        BitsError {
            field_path: path,
            field_path_len: len,
            bit_start: self.bit_start.saturating_add(bit_start as u8),
            ..self
        }
    }

    /// Shift the reported bit range, e.g. for an array element.
    #[doc(hidden)]
    pub const fn at_offset(self, bit_start: usize) -> Self {
        BitsError {
            bit_start: self.bit_start.saturating_add(bit_start as u8),
            ..self
        }
    }

    fn write_field_path(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let path = self.field_path();
        for (i, name) in path.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            f.write_str(name)?;
        }
        Ok(())
    }
}

impl fmt::Debug for BitsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BitsError")
            .field("type_name", &self.type_name)
            .field("field_path", &self.field_path())
            .field("invalid_bits", &self.invalid_bits)
            .field("bitsize", &self.bitsize)
            .field("bit_start", &self.bit_start)
            .finish()
    }
}

impl fmt::Display for BitsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "`{}` has no representation for 0b{:0width$b}",
            self.type_name,
            self.invalid_bits,
            width = self.bitsize as usize,
        )?;
        if self.field_path_len != 0 || self.bit_start != 0 {
            write!(f, " (")?;
            if self.field_path_len != 0 {
                f.write_str("field `")?;
                self.write_field_path(f)?;
                f.write_str("`, ")?;
            }
            write!(f, "bits {}..={})", self.bit_start, self.bit_end())?;
        }
        Ok(())
    }
}

impl core::error::Error for BitsError {}

/// Internally used for generating the `Result::Err` type in `TryFrom`.
#[doc(hidden)]
pub const fn give_me_error(type_name: &'static str, invalid_bits: u128, bitsize: u8) -> BitsError {
    BitsError {
        type_name,
        field_path: [""; MAX_FIELD_PATH],
        field_path_len: 0,
        invalid_bits,
        bitsize,
        bit_start: 0,
    }
}

/// Convert a field's `TryFrom` error into [`BitsError`].
///
/// Nested `#[bitsize]` types keep their `BitsError` (inner path intact).
/// `Infallible` never happens (`FromBits` fields).
/// There is no blanket impl since a custom `TryFrom::Error` needs its own impl.
#[doc(hidden)]
pub trait IntoBitsError {
    fn into_bits_error(self, type_name: &'static str, invalid_bits: u128, bitsize: u8) -> BitsError;
}

impl IntoBitsError for BitsError {
    fn into_bits_error(self, _: &'static str, _: u128, _: u8) -> BitsError {
        self
    }
}

impl IntoBitsError for core::convert::Infallible {
    fn into_bits_error(self, _: &'static str, _: u128, _: u8) -> BitsError {
        match self {}
    }
}

/// Only basing this on Integer did not work, as bool and others are not Integer.
/// We could remove the whole macro_rules thing if it worked, though.
/// Maybe there is some way to do this, I'm not deep into types.
/// Finding some way to combine Integer and Bitsized would be good as well.
impl<BaseType, const BITS: usize> Bitsized for arbitrary_int::UInt<BaseType, BITS>
where
    BaseType: arbitrary_int::traits::UnsignedInteger + arbitrary_int::traits::BuiltinInteger,
    arbitrary_int::UInt<BaseType, BITS>: arbitrary_int::traits::UnsignedInteger,
{
    type ArbitraryInt = Self;
    const BITS: usize = BITS;
    const MAX: Self::ArbitraryInt = <Self as arbitrary_int::traits::Integer>::MAX;
    #[inline]
    fn as_int(&self) -> Self::ArbitraryInt {
        *self
    }
}

impl<BaseType, const BITS: usize> Bitsized for arbitrary_int::Int<BaseType, BITS>
where
    BaseType: arbitrary_int::traits::SignedInteger + arbitrary_int::traits::BuiltinInteger,
    arbitrary_int::Int<BaseType, BITS>: arbitrary_int::traits::SignedInteger,
{
    type ArbitraryInt = Self;
    const BITS: usize = BITS;
    const MAX: Self::ArbitraryInt = <Self as arbitrary_int::traits::Integer>::MAX;
    #[inline]
    fn as_int(&self) -> Self::ArbitraryInt {
        *self
    }
}

macro_rules! bitsized_impl {
    ($(($name:ident, $bits:expr)),+) => {
        $(
            impl Bitsized for $name {
                type ArbitraryInt = Self;
                const BITS: usize = $bits;
                const MAX: Self::ArbitraryInt = <Self as arbitrary_int::traits::Integer>::MAX;
                #[inline]
                fn as_int(&self) -> Self::ArbitraryInt {
                    *self
                }
            }
        )+
    };
}
bitsized_impl!((u8, 8), (u16, 16), (u32, 32), (u64, 64), (u128, 128));
bitsized_impl!((i8, 8), (i16, 16), (i32, 32), (i64, 64), (i128, 128));

/// Handle bool as a u1
impl Bitsized for bool {
    type ArbitraryInt = arbitrary_int::u1;
    const BITS: usize = 1;
    const MAX: Self::ArbitraryInt = <arbitrary_int::u1 as arbitrary_int::traits::Integer>::MAX;
    #[inline]
    fn as_int(&self) -> Self::ArbitraryInt {
        arbitrary_int::u1::new(*self as u8)
    }
}
