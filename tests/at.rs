#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs, const_maybe_uninit_write))]

use bilge::prelude::*;

/// Datasheet-style holes: bit 3 and bits 8..=11, the rest is implicit padding.
#[bitsize(16)]
#[derive(Clone, Copy, DebugBits, PartialEq, FromBits, DefaultBits, BinaryBits)]
struct Status {
    #[at(3)]
    pub ready: bool,
    #[at(8..=11)]
    pub nack: u4,
}

#[bitsize(16)]
#[derive(FromBits, DebugBits, PartialEq)]
struct ContinuesAfterAt {
    #[at(8)]
    high: u4,
    low: u4,
}

#[bitsize(16)]
#[derive(FromBits, DebugBits, PartialEq)]
struct SequentialThenAt {
    low: u4,
    #[at(8)]
    high: u4,
}

#[bitsize(8)]
#[derive(DebugBits, PartialEq, TryFromBits)]
struct ClassAtBitThree {
    #[at(3)]
    class: Class,
}

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum Class {
    Mobile,
    Semimobile,
    Stationary = 0x3,
}

#[bitsize(8)]
#[derive(FromBits, DebugBits, PartialEq, DefaultBits)]
struct TupleAt(#[at(3)] bool, #[at(6)] u2);

#[test]
fn at_places_fields_and_leaves_holes() {
    let st = Status::from(u16::new(1 << 3 | (0xA << 8)));
    assert!(st.ready());
    assert_eq!(st.nack(), u4::new(0xA));
    assert_eq!(format!("{st:?}"), "Status { ready: true, nack: 10 }");

    let st = Status::new(true, u4::new(0xA));
    assert_eq!(u16::from(st).value(), 1 << 3 | (0xA << 8));

    let mut st = Status::from(u16::new(0xFFFF));
    st.set_ready(false);
    st.set_nack(u4::new(0));
    // named fields cleared; holes keep their previous bits
    assert_eq!(u16::from(st).value(), 0xF0F7);
}

#[test]
fn at_cursor_continues_sequentially() {
    let v = ContinuesAfterAt::new(u4::new(0xA), u4::new(0x5));
    assert_eq!(v.high(), u4::new(0xA));
    assert_eq!(v.low(), u4::new(0x5));
    assert_eq!(u16::from(v).value(), (0xA << 8) | (0x5 << 12));

    let v = SequentialThenAt::new(u4::new(0xA), u4::new(0x5));
    assert_eq!(v.low(), u4::new(0xA));
    assert_eq!(v.high(), u4::new(0x5));
    assert_eq!(u16::from(v).value(), 0xA | (0x5 << 8));
}

#[test]
fn try_from_validates_the_placed_bits() {
    // class occupies bits 3..=4; 0b11 there is Stationary
    let ok = ClassAtBitThree::try_from(u8::new(0b0001_1000)).unwrap();
    assert_eq!(ok.class(), Class::Stationary);

    // 0b10 in those bits is the gap in Class
    let err = ClassAtBitThree::try_from(u8::new(0b0001_0000));
    assert!(err.is_err());

    // hole bits must not be mistaken for the enum
    let ok = ClassAtBitThree::try_from(u8::new(0b0000_0010)).unwrap();
    assert_eq!(ok.class(), Class::Mobile);
}

#[test]
fn default_and_binary_include_holes() {
    let st = Status::default();
    assert_eq!(st, Status::new(false, u4::new(0)));
    assert_eq!(format!("{st:b}"), "0000_0000_0000_0_000");

    let st = Status::new(true, u4::new(0xA));
    assert_eq!(format!("{st:b}"), "0000_1010_0000_1_000");
}

#[test]
fn tuple_struct_at() {
    let t = TupleAt::new(true, u2::new(0b11));
    assert!(t.val_0());
    assert_eq!(t.val_1(), u2::new(0b11));
    assert_eq!(u8::from(t).value(), (1 << 3) | (0b11 << 6));
}
