#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

use bilge::prelude::*;

#[bitsize(8)]
#[derive(FromBits, DebugBits, PartialEq, Clone, Copy)]
struct CrtcIndex {
    pub index: u8,
}

#[bitsize(8)]
#[derive(FromBits, DebugBits, PartialEq, Clone, Copy)]
struct HorizontalDisplayEnd {
    pub width_minus_1: u8,
}

#[bitsize(8)]
#[derive(FromBits, DebugBits, PartialEq, Clone, Copy)]
struct MaxScanLine {
    pub max_scan: u5,
    padding: u3,
}

#[bitsize(8)]
#[discriminant(CrtcIndex)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy, BinaryBits)]
enum CrtcReg {
    Horiz(HorizontalDisplayEnd) = 0x01,
    MaxScan(MaxScanLine) = 0x09,
}

#[bitsize(8)]
#[discriminant(u1)]
#[derive(FromBits, Debug, PartialEq, Clone, Copy)]
enum Port {
    Lo(u8) = 0,
    Hi(u8) = 1,
}

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum HaveFun {
    Yes,
    No,
    Maybe,
}

#[bitsize(2)]
#[discriminant(u2)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum TaggedUnfilled {
    Cool(HaveFun) = 0,
    Lame(HaveFun) = 1,
}

#[test]
fn crtc_roundtrip() {
    let reg = CrtcReg::try_from((CrtcIndex::from(0x01), 79)).unwrap();
    assert_eq!(reg, CrtcReg::Horiz(HorizontalDisplayEnd::from(79)));
    let (index, data) = reg.to_tag_and_data();
    assert_eq!(index.index(), 0x01);
    assert_eq!(data, 79);

    let reg = CrtcReg::MaxScan(MaxScanLine::from(0b000_01111));
    let (index, data): (CrtcIndex, u8) = reg.into();
    assert_eq!(CrtcReg::try_from((index, data)).unwrap(), reg);

    let err = CrtcReg::try_from((CrtcIndex::from(0x00), 0)).unwrap_err();
    assert_eq!(err.type_name(), "CrtcReg");
    assert_eq!(err.invalid_bits(), 0);
    assert_eq!(err.bitsize(), 8);
    assert_eq!(format!("{:b}", CrtcReg::Horiz(HorizontalDisplayEnd::from(0b1010_0101))), "10100101");
}

#[test]
fn filled_tag_is_from() {
    let p = Port::from((u1::new(1), 0xAB));
    assert_eq!(p, Port::Hi(0xAB));
    assert_eq!(p.to_tag_and_data(), (u1::new(1), 0xAB));
    assert_eq!(Port::from((u1::new(0), 0x00)), Port::Lo(0x00));
}

#[test]
fn unfilled_payload_and_tag() {
    let ok = TaggedUnfilled::try_from((u2::new(0), u2::new(0))).unwrap();
    assert_eq!(ok, TaggedUnfilled::Cool(HaveFun::Yes));

    let payload_err = TaggedUnfilled::try_from((u2::new(0), u2::new(3))).unwrap_err();
    assert_eq!(payload_err.type_name(), "HaveFun");
    assert_eq!(payload_err.invalid_bits(), 3);
    assert_eq!(payload_err.bitsize(), 2);

    let tag_err = TaggedUnfilled::try_from((u2::new(2), u2::new(0))).unwrap_err();
    assert_eq!(tag_err.type_name(), "TaggedUnfilled");
    assert_eq!(tag_err.invalid_bits(), 2);
    assert_eq!(tag_err.bitsize(), 2);

    assert_eq!(ok.to_tag_and_data(), (u2::new(0), u2::new(0)));
}
