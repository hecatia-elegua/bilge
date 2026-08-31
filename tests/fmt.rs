#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
#![allow(clippy::unusual_byte_groupings)]

use bilge::prelude::*;

#[bitsize(10)]
#[derive(FromBits, BinaryBits, PartialEq, Debug)]
enum Bangers {
    Italian,
    Bratwurst,
    #[fallback]
    Chorizo(u10),
}

#[bitsize(2)]
#[derive(FromBits, BinaryBits)]
enum Mash {
    Potatoes,
    #[fallback]
    Peas,
}

#[bitsize(52)]
#[derive(FromBits, BinaryBits)]
struct Register {
    reserved: u13,
    reg1: bool,
    reg2: u16,
    reserved: u4,
    reg3: u18,
}

#[bitsize(64)]
#[derive(FromBits, BinaryBits)]
struct Lunch(Bangers, Mash, Register);

#[test]
fn binary_formatting() {
    let b = u10::new(0b1100110011).into();
    let m = u2::new(0b00).into();
    let reg = u52::new(0b110010110010101001_1011_1011011001100011_1_1011001100000).into();

    let lunch = Lunch::new(b, m, reg);

    // fallback value is used
    assert_eq!(format!("0b{:b}", lunch.val_0()), "0b1100110011");

    // output matches u16's output
    let bang_raw: u16 = 0b1100110011;
    let bang = Bangers::from(u10::new(bang_raw));
    assert_eq!(bang, lunch.val_0());
    assert_eq!(format!("0b{:b}", lunch.val_0()), format!("0b{:b}", bang_raw));

    // padding is respected
    assert_eq!(format!("0b{:b}", lunch.val_1()), "0b00");

    // this one has underscores
    assert_eq!(
        format!("0b{:b}", lunch.val_2()),
        "0b110010110010101001_1011_1011011001100011_1_1011001100000"
    );

    // ...but the underscores are not "inherited" for the nested struct
    assert_eq!(
        format!("0b{:b}", lunch),
        "0b1100101100101010011011101101100110001111011001100000_00_1100110011"
    );
}

#[bitsize(8)]
#[derive(FromBits, PartialEq, DebugBits)]
struct NoCloneByte {
    raw: u8,
}

#[bitsize(8)]
#[discriminant(u1)]
#[derive(FromBits, PartialEq, Debug, BinaryBits)]
enum TaggedNoClone {
    Lo(NoCloneByte) = 0,
    Hi(NoCloneByte) = 1,
}

#[bitsize(2)]
#[derive(FromBits, PartialEq, DebugBits)]
struct NoClonePair {
    a: bool,
    b: bool,
}

#[bitsize(4)]
#[discriminant_at(0..=1)]
#[derive(FromBits, PartialEq, Debug, BinaryBits)]
enum PackedNoClone {
    A(NoClonePair) = 0,
    B(NoClonePair) = 1,
    C(NoClonePair) = 2,
    D(NoClonePair) = 3,
}

#[test]
fn binary_bits_does_not_need_clone_on_payloads() {
    let tagged = TaggedNoClone::Hi(NoCloneByte::from(0b1010_0101));
    assert_eq!(format!("{tagged:b}"), "10100101");
    assert_eq!(tagged.to_tag_and_data(), (u1::new(1), 0b1010_0101));

    let packed = PackedNoClone::C(NoClonePair::new(true, false));
    // tag 0b10 in bits 0..=1, payload a=1 b=0 in bits 2..=3 → 0b01_10
    assert_eq!(format!("{packed:b}"), "0110");
    assert_eq!(u4::from(packed), u4::new(0b01_10));
}

#[test]
fn as_int_matches_from_for_value_fallback() {
    let e = Bangers::from(u10::new(0b1100110011));
    assert_eq!(e.as_int(), u10::from(e));
}

#[bitsize(2)]
#[non_exhaustive]
#[derive(TryFromBits, BinaryBits, Debug, PartialEq)]
enum OpenSet {
    A,
    B,
}

#[test]
fn binary_bits_allows_non_exhaustive_tryfrom_enum() {
    let v = OpenSet::try_from(u2::new(0)).unwrap();
    assert_eq!(v, OpenSet::A);
    assert_eq!(format!("{v:b}"), "00");
}
