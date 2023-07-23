#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![no_std]

#[doc(no_inline)]
pub use arbitrary_int;
pub use bilge_impl::{bitsize, bitsize_internal, DebugBits, FromBits, TryFromBits, BinaryBits};

/// used for `use bilge::prelude::*;`
pub mod prelude {
    #[doc(no_inline)]
    pub use super::{
        bitsize, bitsize_internal,
        Bitsized,
        DebugBits, FromBits, TryFromBits, BinaryBits,
        // we control the version, so this should not be a problem
        arbitrary_int::*,
        Filled, assume_filled
    };
}

/// This is internally used, but might be useful. No guarantees are given (for now).
pub trait Bitsized {
    type ArbitraryInt;
    const BITS: usize;
    const MAX: Self::ArbitraryInt;
}

/// Internally used marker trait. 
/// # Safety 
/// 
/// Avoid implementing this for your types. Implementing this trait could break invariants.
pub unsafe trait Filled: Bitsized {}
unsafe impl<T> Filled for T where T: Bitsized + From<<T as Bitsized>::ArbitraryInt> {}

/// This is generated to statically validate that a type implements `FromBits`.
pub fn assume_filled<T: Filled>() {}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub struct BitsError;

/// Internally used for generating the `Result::Err` type in `TryFrom`.
/// 
/// This is needed since we don't want users to be able to create `BitsError` right now.
/// We'll be able to turn `BitsError` into an enum later, or anything else really.
pub fn give_me_error() -> BitsError {
    BitsError
}

/// This is used in `bitsize.rs` gen, for showing a compile error.
/// Only needed because const contexts can't use `format!` and `COMPUTED_SIZE` is not a plain number.
/// `evaluation of `bilge::AssertEquals::<19, 18>::EQUAL` failed` will tell you what's wrong.
pub struct AssertEquals<const COMPUTED_SIZE: usize, const DECLARED_SIZE: usize>;
impl<const COMPUTED_SIZE: usize, const DECLARED_SIZE: usize> AssertEquals<COMPUTED_SIZE, DECLARED_SIZE> {
    pub const EQUAL: () = assert!(COMPUTED_SIZE == DECLARED_SIZE, "computed bitsize and declared bitsize differ");
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
}

macro_rules! bitsized_impl {
    ($(($name:ident, $bits:expr)),+) => {
        $(
            impl Bitsized for $name {
                type ArbitraryInt = Self;
                const BITS: usize = $bits;
                const MAX: Self::ArbitraryInt = <Self as arbitrary_int::Number>::MAX;
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
}
