#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

use assert_matches::assert_matches;
use bilge::prelude::*;

#[bitsize(7)]
#[derive(FromBits, Debug)]
enum UnitFallback {
    First,
    Second,
    Third,
    #[fallback]
    Reserved,
}

#[bitsize(7)]
#[derive(FromBits, Debug)]
enum FallbackWithValue {
    First,
    Second,
    Third,
    #[fallback]
    Reserved(u7),
}

fn main() {
    // with unit fallback variants, converting back to a number will discard the fallback value,
    // if that value is not the variant's ordinal:
    let original = u7::new(7);
    let converted = UnitFallback::from(original);
    assert_matches!(converted, UnitFallback::Reserved);
    assert_eq!(u7::from(converted), u7::new(3));

    //...but this is not true for fallback variants with a value:
    let original = u7::new(3);
    let converted = FallbackWithValue::from(original);
    assert_eq!(u7::from(converted), original);

    let original = u7::new(9);
    let converted = FallbackWithValue::from(original);
    assert_eq!(u7::from(converted), original);
}
