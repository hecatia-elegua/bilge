#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

use bilge::prelude::*;
use assert_matches::assert_matches;

#[bitsize(7)]
#[derive(FromBits, Debug)]
enum HasFallback {
    First,
    Second,
    Third,
    #[fallback]
    Reserved,
}

#[bitsize(7)]
#[derive(FromBits, Debug)]
enum HasFallbackOnNonReserved {
    First,
    #[fallback]
    Second,
    Third,
}

#[bitsize(7)]
#[derive(FromBits, Debug)]
enum HasFallbackWithValue {
    First,
    Second,
    Third,
    #[fallback]
    Reserved(u7),
}

#[bitsize(7)]
#[derive(FromBits, Debug)]
enum HasFallbackAndNonDefaultOrdinals {
    First = 1,
    Second = 3,
    Third = 5,
    #[fallback]
    Reserved,
}

fn main() {
    assert_matches!(
        HasFallback::from(u7::new(5)), 
        HasFallback::Reserved
    );

    assert_matches!(
        HasFallbackOnNonReserved::from(u7::new(1)), 
        HasFallbackOnNonReserved::Second
    );
    assert_matches!(
        HasFallbackOnNonReserved::from(u7::new(9)), 
        HasFallbackOnNonReserved::Second
    );

    assert_matches!(
        HasFallbackWithValue::from(u7::new(0b11)), 
        HasFallbackWithValue::Reserved(n) if n == u7::new(3)
    );
    assert_matches!(
        HasFallbackWithValue::from(u7::new(0b1001)), 
        HasFallbackWithValue::Reserved(n) if n == u7::new(9)
    );

    assert_matches!(
        HasFallbackAndNonDefaultOrdinals::from(u7::new(0)), 
        HasFallbackAndNonDefaultOrdinals::Reserved
    );
    assert_matches!(
        HasFallbackAndNonDefaultOrdinals::from(u7::new(5)), 
        HasFallbackAndNonDefaultOrdinals::Third
    );

    // with unit fallback variants, converting back to a number will discard the fallback value,
    // if that value is not the variant's ordinal:
    let original = u7::new(7);
    let converted = HasFallback::from(original);
    assert_matches!(converted, HasFallback::Reserved);
    assert_eq!(u7::from(converted), u7::new(3));

    //...but this is not true for fallback variants with a value:
    let original = u7::new(3);
    let converted = HasFallbackWithValue::from(original);
    assert_eq!(u7::from(converted), original);
    
    let original = u7::new(9);
    let converted = HasFallbackWithValue::from(original);
    assert_eq!(u7::from(converted), original);
}