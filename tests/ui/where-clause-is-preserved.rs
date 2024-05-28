#![feature(const_trait_impl)]

use bilge::prelude::*;
use core::marker::PhantomData;

#[derive(Default)]
struct Generic<T>(u2, PhantomData<T>);

impl<T> Bitsized for Generic<T> {
    type ArbitraryInt = u2;
    const BITS: usize = 2;
    const MAX: Self::ArbitraryInt = <u2 as Bitsized>::MAX;
}

impl<T> const From<u2> for Generic<T> {
    fn from(val: u2) -> Self {
        Self(val, PhantomData)
    }
}

impl<T> const From<Generic<T>> for u2 {
    fn from(val: Generic<T>) -> u2 {
        val.0
    }
}

#[bitsize(5)]
struct BoundedGeneric<T>(Generic<T>, u3)
where
    T: Iterator<Item = ()>;

fn main() {
    // kinda ick, but the only way I could find to check that the clause was on the struct directly
    BoundedGeneric::<()> {
        value: <_>::default(),
        _phantom: PhantomData,
    };
    // check that where clause is present on inherent impl
    BoundedGeneric::<()>::new(<_>::default(), u3::new(0));
}
