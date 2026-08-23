#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

use bilge::prelude::*;

#[bitsize(2)]
#[discriminant_at(0)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy)]
enum Flagged {
    Off(bool) = 0,
    On(bool) = 1,
}

#[bitsize(4)]
#[discriminant_at(2..=3)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy, BinaryBits)]
enum HighTag {
    A(u2) = 0,
    B(u2) = 1,
    C(u2) = 2,
    D(u2) = 3,
}

#[bitsize(4)]
#[discriminant_at(0..=1)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy, BinaryBits)]
enum LowTag {
    A(u2) = 0,
    B(u2) = 1,
    C(u2) = 2,
    D(u2) = 3,
}

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum HaveFun {
    Yes,
    No,
    Maybe,
}

#[bitsize(4)]
#[discriminant_at(0..=1)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum TaggedUnfilled {
    Cool(HaveFun) = 0,
    Lame(HaveFun) = 1,
}

#[test]
fn bool_tag_roundtrip() {
    let v = Flagged::On(true);
    let raw = u2::from(v);
    assert_eq!(raw.value(), 0b11);
    assert_eq!(Flagged::from(raw), v);

    let v = Flagged::Off(false);
    assert_eq!(u2::from(v).value(), 0b00);
    assert_eq!(Flagged::from(u2::new(0b00)), v);
    assert_eq!(Flagged::from(u2::new(0b10)), Flagged::Off(true));
}

#[test]
fn msb_tag_roundtrip() {
    // bits 0..=1 payload, 2..=3 tag
    let v = HighTag::C(u2::new(0b01));
    let raw = u4::from(v);
    assert_eq!(raw.value(), 0b10_01);
    assert_eq!(HighTag::from(raw), v);
    assert_eq!(format!("{v:b}"), "1001");
}

#[test]
fn lsb_tag_roundtrip() {
    // bits 0..=1 tag, 2..=3 payload
    let v = LowTag::C(u2::new(0b01));
    let raw = u4::from(v);
    assert_eq!(raw.value(), 0b01_10);
    assert_eq!(LowTag::from(raw), v);
    assert_eq!(format!("{v:b}"), "0110");
}

#[test]
fn unfilled_payload_and_tag() {
    let ok = TaggedUnfilled::try_from(u4::new(0b00_00)).unwrap();
    assert_eq!(ok, TaggedUnfilled::Cool(HaveFun::Yes));

    // HaveFun has no discriminant 3
    assert!(TaggedUnfilled::try_from(u4::new(0b11_00)).is_err());
    // tag 2 is unused
    assert!(TaggedUnfilled::try_from(u4::new(0b00_10)).is_err());

    assert_eq!(u4::from(ok).value(), 0);
}
