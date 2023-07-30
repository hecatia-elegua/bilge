#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

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
enum UnitFoo {
    #[fallback]
    Foo,
    Bar,
    Baz,
}

#[bitsize(5)]
#[derive(FromBits, Debug)]
enum UnitBar {
    Foo,
    #[fallback]
    Bar,
    Baz,
}

#[bitsize(5)]
#[derive(FromBits, Debug)]
enum UnitBaz {
    Foo,
    Bar,
    #[fallback]
    Baz,
}

#[test]
fn different_unit_fallback_positions() {
    let val = u5::new(4);

    assert_matches!(UnitFoo::from(val), UnitFoo::Foo);
    assert_eq!(u5::from(UnitFoo::Foo), u5::new(0));

    assert_matches!(UnitBar::from(val), UnitBar::Bar);
    assert_eq!(u5::from(UnitBar::Bar), u5::new(1));

    assert_matches!(UnitBaz::from(val), UnitBaz::Baz);
    assert_eq!(u5::from(UnitBaz::Baz), u5::new(2));
}
