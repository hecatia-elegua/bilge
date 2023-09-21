#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
use bilge::prelude::*;

#[bitsize(4)]
#[derive(DebugBits, TryFromBits, Clone, Copy)]
struct IncompleteStruct {
    field1: u2,
    field2: CompleteEnum,
}
#[bitsize(2)]
#[derive(Debug, FromBits, PartialEq, Eq)]
enum CompleteEnum {
    A = 0,
    B = 1,
    C = 0b0000_0010,
    D = 3,
}

#[bitsize(32, manual)]
#[derive(DebugBits, TryFromBits, BinaryBits)]
struct ManualLayout1 {
    #[bits(3..=5)]
    a: u3,
    #[bit(6)]
    b: bool,
    #[bits(10..14)]
    c: [u2; 2],
    #[bits(18..22)]
    d: (u1, u3),
    #[bits(28..32)]
    e: IncompleteStruct,
}

// Order should not matter
#[bitsize(32, manual)]
#[derive(DebugBits, TryFromBits, BinaryBits)]
struct ManualLayout2 {
    #[bits(28..32)]
    e: IncompleteStruct,
    #[bits(3..=5)]
    a: u3,
    #[bits(10..14)]
    c: [u2; 2],
    #[bits(18..22)]
    d: (u1, u3),
    #[bit(6)]
    b: bool,
}

fn main() {
    let a = u3::new(0b110);
    let b = true;
    let c = [u2::new(0b10); 2];
    let d = (u1::new(1), u3::new(0b011));
    let e = IncompleteStruct::new(u2::new(0b11), CompleteEnum::C);
    let m1_new = ManualLayout1::new(a, b, c, d, e);
    assert_eq!(a, m1_new.a());
    assert_eq!(b, m1_new.b());
    assert_eq!(c, m1_new.c());
    assert_eq!(d, m1_new.d());
    assert_eq!(e.field1(), m1_new.e().field1());
    assert_eq!(e.field2(), m1_new.e().field2());
    //                   e            d          c       b  a
    //                 /   \        /   \      /   \     | / \
    let raw: u32 = 0b_10_11_000000_011_1_0000_10_10_000_1_110_000;
    let m1_raw = ManualLayout1::try_from(u32::new(raw)).unwrap();
    let m2_raw = ManualLayout2::try_from(u32::new(raw)).unwrap();
    assert_eq!(m1_new.value, m1_raw.value);
    assert_eq!(m1_raw.value, m2_raw.value);
}
