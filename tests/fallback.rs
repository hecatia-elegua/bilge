#![cfg_attr(
    feature = "nightly",
    feature(const_convert, const_trait_impl, const_mut_refs)
)]

use assert_matches::assert_matches;
use bilge::prelude::*;

#[bitsize(11)]
#[derive(FromBits, Debug)]
enum BestPet {
    Cat,
    Dog,
    #[fallback]
    Parrot(u11),
}

#[test]
fn fallback_value_is_preserved() {
    let max = BestPet::MAX.value();

    for value in 0..max {
        let original = u11::new(value);
        let converted = BestPet::from(original);
        let inverse = u11::from(converted);

        assert_eq!(original, inverse);
    }
}

#[bitsize(8)]
#[derive(FromBits, Debug)]
enum NonDef {
    A = 1,
    B = 3,
    C = 5,
    #[fallback]
    D,
}

#[test]
fn non_default_ordinals() {
    assert_matches!(NonDef::from(0_u8), NonDef::D);
    assert_matches!(NonDef::from(5_u8), NonDef::C);
    assert_eq!(u8::from(NonDef::D), 6_u8);
}

#[bitsize(5)]
#[derive(FromBits, Debug)]
enum FinlvUnit {
    Foo,
    #[fallback]
    Bar,
    Baz,
}

#[bitsize(5)]
#[derive(FromBits, Debug)]
enum FinlvValue {
    Foo,
    #[fallback]
    Bar(u5),
    Baz,
}

#[test]
fn when_fallback_is_not_last_variant() {
    assert_matches!(FinlvUnit::from(u5::new(4)), FinlvUnit::Bar);
    assert_eq!(u5::from(FinlvUnit::Bar), u5::new(1));

    let seven = u5::new(7);
    assert_matches!(
        FinlvValue::from(seven),
        FinlvValue::Bar(n) if n == seven
    );
    assert_eq!(u5::from(FinlvValue::Bar(seven)), seven);

    let one = u5::new(1);
    assert_matches!(
        FinlvValue::from(one),
        FinlvValue::Bar(n) if n == one
    );
    assert_eq!(u5::from(FinlvValue::Bar(one)), one);
}
