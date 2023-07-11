#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![no_std]

#[doc(no_inline)]
pub use arbitrary_int;
pub use bilge_impl::{bitsize, bitsize_internal, DebugBits, FromBits, TryFromBits};

/// used for `use bilge::prelude::*;`
pub mod prelude {
    #[doc(no_inline)]
    pub use super::{
        bitsize, bitsize_internal,
        Bitsized,
        DebugBits, FromBits, TryFromBits,
        // we control the version, so this should not be a problem
        arbitrary_int::*,
    };
}

/// This is internally used, but might be useful. No guarantees are given (for now).
pub trait Bitsized {
    type ArbitraryInt;
    const BITS: usize;
    const MAX: Self::ArbitraryInt;
    const FILLED: bool;
}

/// Only basing this on Number did not work, as bool and others are not Number.
/// We could remove the whole macro_rules thing if it worked, though.
/// Maybe there is some way to do this, I'm not deep into types.
/// Finding some way to combine Number and Bitsized would be good as well.
impl<BaseType, const BITS: usize> Bitsized for arbitrary_int::UInt<BaseType, BITS>
where
    arbitrary_int::UInt<BaseType, BITS>: arbitrary_int::Number
{
    type ArbitraryInt = Self;
    const BITS: usize = BITS;
    const MAX: Self::ArbitraryInt = <Self as arbitrary_int::Number>::MAX;
    const FILLED: bool = true;
}

macro_rules! bitsized_impl {
    ($(($name:ident, $bits:expr)),+) => {
        $(
            impl Bitsized for $name {
                type ArbitraryInt = Self;
                const BITS: usize = $bits;
                const MAX: Self::ArbitraryInt = <Self as arbitrary_int::Number>::MAX;
                const FILLED: bool = true;
            }
        )+
    };
}
bitsized_impl!((u8, 8), (u16, 16), (u32, 32), (u64, 64), (u128, 128));

/// Handle bool as a u1
impl Bitsized for bool {
    type ArbitraryInt = arbitrary_int::u1;
    const BITS: usize = 1;
    const MAX: Self::ArbitraryInt = <arbitrary_int::u1 as arbitrary_int::Number>::MAX;
    const FILLED: bool = true;
}
