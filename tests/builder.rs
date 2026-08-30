#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
use bilge::prelude::*;

#[bitsize(32)]
#[derive(BuilderBits, FromBits, PartialEq, DebugBits)]
struct AddInstruction {
    src1: u4,
    reserved: u4,
    #[default(u6::new(8))]
    opcode: u6,
    src2: u4,
    reserved: u2,
    dst: u4,
    reserved: u8,
}

#[bitsize(4)]
#[derive(BuilderBits, FromBits, PartialEq, DebugBits)]
struct TuplePair(bool, u3);

#[bitsize(4)]
#[derive(BuilderBits, FromBits, PartialEq, DebugBits)]
struct RequiredFlags {
    flags: [bool; 4],
}

const FLAGS: [bool; 4] = [true, false, true, false];

#[bitsize(4)]
#[derive(BuilderBits, DefaultBits, FromBits, PartialEq, DebugBits)]
struct OptionalFlags {
    #[default(FLAGS)]
    flags: [bool; 4],
}

#[test]
fn named_exactly_once() {
    let src1 = u4::new(1);
    let src2 = u4::new(2);
    let dst = u4::new(3);
    let add = AddInstruction::builder().src1(src1).src2(src2).dst(dst).build();
    assert_eq!(add, AddInstruction::new(src1, u6::new(8), src2, dst));
}

#[test]
fn optional_default_can_be_overridden() {
    let add = AddInstruction::builder()
        .opcode(u6::new(1))
        .src1(u4::new(0))
        .src2(u4::new(0))
        .dst(u4::new(0))
        .build();
    assert_eq!(add.opcode(), u6::new(1));
}

#[test]
fn tuple_struct_uses_val_n() {
    let pair = TuplePair::builder().val_0(true).val_1(u3::new(5)).build();
    assert_eq!(pair, TuplePair::new(true, u3::new(5)));
}

#[test]
fn required_array_must_be_set() {
    let flags = [true, false, true, false];
    let bits = RequiredFlags::builder().flags(flags).build();
    assert_eq!(bits, RequiredFlags::new(flags));
}

#[test]
fn optional_array_uses_default_expr() {
    let bits = OptionalFlags::builder().build();
    assert_eq!(bits.flags(), FLAGS);
    assert_eq!(bits, OptionalFlags::default());

    let bits = OptionalFlags::builder().flags([true; 4]).build();
    assert_eq!(bits.flags(), [true; 4]);
}

#[bitsize(8)]
#[derive(BuilderBits, FromBits, PartialEq, DebugBits)]
struct EmptyBuilder {
    reserved: u8,
}

#[test]
fn builder_with_no_fields() {
    let bits = EmptyBuilder::builder().build();
    assert_eq!(bits, EmptyBuilder::new());
}

#[bitsize(8)]
#[derive(BuilderBits, DefaultBits, FromBits, PartialEq, DebugBits)]
struct OnlyDefault {
    #[default(u8::new(7))]
    val: u8,
}

#[test]
fn builder_with_only_a_default_field() {
    let bits = OnlyDefault::builder().build();
    assert_eq!(bits.val(), 7);
    assert_eq!(bits, OnlyDefault::default());

    let bits = OnlyDefault::builder().val(u8::new(1)).build();
    assert_eq!(bits.val(), 1);
}
