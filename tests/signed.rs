#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs, const_maybe_uninit_write))]
use bilge::prelude::*;

#[bitsize(27)]
#[derive(FromBits, PartialEq, DebugBits)]
struct BitStructSigned {
    x: i20,
    y: i7,
}

#[test]
fn bit_struct_signed() {
    let mut bits = BitStructSigned::from(u27::new(0b1010110_00110001000101000001));
    let new = BitStructSigned::new(i20::new(201025), i7::new(-42));
    let x = i20::new(201025);
    let y = i7::new(-42);

    assert_eq!(bits, new);
    assert_eq!(bits.x(), x);
    assert_eq!(bits.y(), y);

    bits.set_y(i7::new(0b0101010));
    let y = i7::new(42);
    assert_eq!(bits.y(), y);
}

#[bitsize(32)]
#[derive(DebugBits, FromBits)]
struct TupleStructSigned(u2, i6, i7, u8, u8, i1);

#[test]
fn tuple_struct_signed() {
    let val_0 = u2::new(0);
    let val_1 = i6::new(-30);
    let val_2 = i7::new(-9);
    let val_3 = u8::new(3);
    let val_4 = u8::new(4);
    let val_5 = i1::new(-1);
    let mut a = TupleStructSigned::new(val_0, val_1, val_2, val_3, val_4, val_5);
    assert_eq!(a.val_0(), val_0);
    assert_eq!(a.val_1(), val_1);
    assert_eq!(a.val_2(), val_2);
    assert_eq!(a.val_3(), val_3);
    assert_eq!(a.val_4(), val_4);
    assert_eq!(a.val_5(), val_5);
    let val_1 = i6::new(25);
    a.set_val_1(val_1);
    assert_eq!(a.val_0(), val_0);
    assert_eq!(a.val_1(), val_1);
    assert_eq!(a.val_2(), val_2);
    assert_eq!(a.val_3(), val_3);
    assert_eq!(a.val_4(), val_4);
    assert_eq!(a.val_5(), val_5);
    let val_2 = i7::MIN;
    a.set_val_2(val_2);
    assert_eq!(a.val_0(), val_0);
    assert_eq!(a.val_1(), val_1);
    assert_eq!(a.val_2(), val_2);
    assert_eq!(a.val_3(), val_3);
    assert_eq!(a.val_4(), val_4);
    assert_eq!(a.val_5(), val_5);
}

#[bitsize(16)]
#[derive(FromBits, DebugBits)]
struct EthercatHeader1Signed {
    len: i11,
    some: i1,
    ty: u4,
}

#[bitsize(16)]
#[derive(FromBits, DebugBits)]
struct EthercatHeader2Signed {
    len: i11,
    reserved: i1,
    ty: u4,
}

#[test]
fn should_be_same_structure_issue_30_signed() {
    let ty = u4::new(0x1);
    let eh1 = EthercatHeader1Signed::new(i11::new(0xe), i1::new(0), ty);
    let eh2 = EthercatHeader2Signed::new(i11::new(0xe), ty);
    assert_eq!(eh1.ty(), ty);
    assert_eq!(eh2.ty(), ty);
    assert_eq!(eh1.value, eh2.value);
}
