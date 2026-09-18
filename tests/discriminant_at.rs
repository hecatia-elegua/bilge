#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
#![allow(clippy::unusual_byte_groupings)]

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
fn tagged_union_repr_matches_tag_width() {
    // 2-bit tag + u2 payload should not use #[repr(u64)] (16 bytes, align 8).
    assert_eq!(core::mem::align_of::<LowTag>(), 1);
    assert!(core::mem::size_of::<LowTag>() <= 2);
    assert_eq!(core::mem::align_of::<Flagged>(), 1);
    assert!(core::mem::size_of::<Flagged>() <= 2);
}

#[test]
fn unfilled_payload_and_tag() {
    let ok = TaggedUnfilled::try_from(u4::new(0b00_00)).unwrap();
    assert_eq!(ok, TaggedUnfilled::Cool(HaveFun::Yes));

    // HaveFun has no discriminant 3
    let payload_err = TaggedUnfilled::try_from(u4::new(0b11_00)).unwrap_err();
    assert_eq!(payload_err.type_name(), "HaveFun");
    assert_eq!(payload_err.invalid_bits(), 3);
    assert_eq!(payload_err.bitsize(), 2);
    assert_eq!(payload_err.bit_start(), 2);
    // tag 2 is unused
    let tag_err = TaggedUnfilled::try_from(u4::new(0b00_10)).unwrap_err();
    assert_eq!(tag_err.type_name(), "TaggedUnfilled");
    assert_eq!(tag_err.invalid_bits(), 2);
    assert_eq!(tag_err.bitsize(), 2);
    assert_eq!(tag_err.bit_start(), 0);

    assert_eq!(u4::from(ok).value(), 0);
}

#[bitsize(8)]
#[discriminant_at(0..=1)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy, BinaryBits)]
enum TaggedFallback {
    A(u6) = 0,
    B(u6) = 1,
    #[fallback]
    Raw(u8),
}

#[bitsize(4)]
#[discriminant_at(0..=1)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy)]
enum AlmostFull {
    A(u2) = 0,
    B(u2) = 1,
    C(u2) = 2,
    #[fallback]
    Other(u4),
}

#[bitsize(8)]
#[discriminant_at(6..=7)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy)]
enum HighFallback {
    A(u6) = 0,
    #[fallback]
    Raw(u8),
}

#[test]
fn value_fallback_keeps_unknown_tags() {
    let known = TaggedFallback::A(u6::new(0b000011));
    let raw = u8::from(known);
    assert_eq!(raw.value(), 0b000011_00);
    assert_eq!(TaggedFallback::from(raw), known);
    assert_eq!(known.as_int(), raw);

    let unknown = u8::new(0b111111_10); // tag 2
    let v = TaggedFallback::from(unknown);
    assert_eq!(v, TaggedFallback::Raw(unknown));
    assert_eq!(u8::from(v), unknown);
    assert_eq!(v.as_int(), unknown);
    assert_eq!(format!("{v:b}"), "11111110");
}

#[test]
fn value_fallback_when_one_tag_is_unused() {
    assert_eq!(AlmostFull::from(u4::new(0b01_10)), AlmostFull::C(u2::new(0b01)));
    let unused_tag = u4::new(0b11_11);
    assert_eq!(AlmostFull::from(unused_tag), AlmostFull::Other(unused_tag));
    assert_eq!(u4::from(AlmostFull::Other(unused_tag)), unused_tag);
}

#[test]
fn value_fallback_with_msb_tag() {
    let known = HighFallback::A(u6::new(0b000001));
    assert_eq!(u8::from(known).value(), 0b00_000001);
    assert_eq!(HighFallback::from(u8::from(known)), known);

    let unknown = u8::new(0b11_000001); // tag 3
    assert_eq!(HighFallback::from(unknown), HighFallback::Raw(unknown));
    assert_eq!(u8::from(HighFallback::Raw(unknown)), unknown);
    assert_eq!(HighFallback::Raw(unknown).as_int(), unknown);
}
