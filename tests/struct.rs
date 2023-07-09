#![allow(clippy::unusual_byte_groupings)]
use bilge::prelude::*;

#[bitsize(1)]
#[derive(FromBits, PartialEq, DebugBits)]
struct Bit {
    inner: u1,
}

#[bitsize(2)]
#[derive(TryFromBits)]
struct UnfilledStruct {
    inner: Unfilled,
}

#[bitsize(2)]
#[derive(TryFromBits, Debug)]
enum Unfilled { A, B, C }

#[test]
fn conversions() {
    // `From` in both directions
    assert_eq!(u1::new(0), u1::from(Bit::new(u1::new(0))));
    assert_eq!(u1::new(1), u1::from(Bit::new(u1::new(1))));
    assert_eq!(Bit::new(u1::new(0)), u1::new(0).into());
    assert_eq!(Bit::new(u1::new(1)), u1::new(1).into());

    // Of course converting to number is always infallible,
    // since the structure describes the bitpattern anyways.
    // `TryFrom<uN>` and `From<Struct>`
    for value in 0..3 {
        let value = u2::new(value);
        let date_activity = UnfilledStruct::try_from(value);
        match date_activity {
            Ok(a) => {
                match a.inner() {
                    Unfilled::A => assert_eq!(u2::new(0u8), value),
                    Unfilled::B => assert_eq!(u2::new(1u8), value),
                    Unfilled::C => assert_eq!(u2::new(2u8), value),
                }
                assert_eq!(u2::from(a), value);
            },
            Err(e) => assert_eq!(e, u2::new(3)),
        }
    }
}

#[bitsize(5)]
#[derive(FromBits, PartialEq, DebugBits)]
struct MultiField {
    field1: u2,
    field2: u1,
    field3: u2,
}

#[test]
fn multiple_fields() {
    // As you can see here, the first bitfield starts at the right.
    let a = MultiField::from(u5::new(0b11_1_01));
    assert_eq!(a.field1(), u2::new(0b01));
    assert_eq!(a.field2(), u1::new(0b1));
    assert_eq!(a.field3(), u2::new(0b11));

    // You can also set fields, of course.
    let mut a = MultiField::from(u5::new(0b00_0_11));
    a.set_field1(u2::new(0b01));
    assert_eq!(a.field1(), u2::new(0b01));
    assert_eq!(a.field2(), u1::new(0b0));
    assert_eq!(a.field3(), u2::new(0b00));
    assert_eq!(a, MultiField::from(u5::new(0b00_0_01)));
    a.set_field2(u1::new(0b1));
    assert_eq!(a.field1(), u2::new(0b01));
    assert_eq!(a.field2(), u1::new(0b1));
    assert_eq!(a.field3(), u2::new(0b00));
    assert_eq!(a, MultiField::from(u5::new(0b00_1_01)));
    a.set_field3(u2::new(0b11));
    assert_eq!(a.field1(), u2::new(0b01));
    assert_eq!(a.field2(), u1::new(0b1));
    assert_eq!(a.field3(), u2::new(0b11));
    assert_eq!(a, MultiField::from(u5::new(0b11_1_01)));

    // Constructors set all fields, ya know?
    let a = MultiField::new(u2::new(0b01), u1::new(0b1), u2::new(0b11));
    assert_eq!(a.field1(), u2::new(0b01));
    assert_eq!(a.field2(), u1::new(0b1));
    assert_eq!(a.field3(), u2::new(0b11));
}

#[bitsize(35)]
#[derive(FromBits, PartialEq, DebugBits)]
struct NestedField {
    nested: Nested,
    field2: u1,
    field3: u2,
}

#[bitsize(32)]
#[derive(FromBits, PartialEq, DebugBits)]
struct Nested {
    field1: u2,
    field2: u8,
    field3: u22,
}

#[test]
fn nested_fields() {
    let a = NestedField::from(u35::new(0b111_1111_1111_1111_1111_0111_1111_1111_1111));
    assert_eq!(a.nested(), Nested::from(0b___1111_1111_1111_1111_0111_1111_1111_1111));
    assert_eq!(a.field2(), u1::new(0b1));
    assert_eq!(a.field3(), u2::new(0b11));

    let nested = a.nested();
    assert_eq!(nested.field1(), u2::new(0b11));
    assert_eq!(nested.field2(), 0b1111_1111);
    assert_eq!(nested.field3(), u22::new(0b11_1111_1111_1111_1101_1111));

    // Currently, setting nested fields works like this
    let mut a = NestedField::from(u35::new(0b111_1111_1111_1111_1111_1111_1111_1111_1111));
    let mut nested = a.nested();
    nested.set_field2(0);
    a.set_nested(nested);
    assert_eq!(a.nested(), Nested::from(0b___1111_1111_1111_1111_1111_1100_0000_0011));

    // Constructors don't do anything different
    let a = NestedField::new(
        Nested::new(u2::new(0b11), 0b1111_1111, u22::new(0b11_0000_1111_0000_1111_1111)),
        u1::new(0b1), u2::new(0b11)
    );
    assert_eq!(a.nested(), Nested::from(0b___1100_0011_1100_0011_1111_1111_1111_1111));
    assert_eq!(a.field2(), u1::new(0b1));
    assert_eq!(a.field3(), u2::new(0b11));
}

/// _hardware hygene is important_
#[bitsize(4)]
#[derive(FromBits)]
struct Bitflags {
    dirty: bool,
    clean: bool,
    smelly: bool,
    tainted: bool,
}

#[test]
fn bools_and_bitflags_like_usage() {
    let mut flags = Bitflags::from(u4::new(0b1101));
    assert!(flags.dirty());
    assert!(!flags.clean());
    assert!(flags.smelly());
    // spray_hardware_with_febreze();
    //             |
    //             v
    flags.set_smelly(false);
    assert!(flags.dirty());
    assert!(!flags.clean());
    assert!(!flags.smelly());
}

#[bitsize(64)]
#[derive(FromBits, PartialEq, DebugBits)]
struct MemoryMappedRegisters {
    reserved: u14,
    status: u2,
    register1: u16,
    reserved: u4,
    register2: u12,
    reserved: u16,
}

#[test]
fn reserved_fields() {
    let mapped = MemoryMappedRegisters::from(0b0000000000000000_001111110000_0000_1000000010001000_11_00000000000000);
    let status = u2::new(0b11);
    let register1 = u16::new(0b1000000010001000);
    let register2 = u12::new(0b001111110000);
    // Reserved fields are skipped in constructors and set to zero automatically
    assert_eq!(mapped, MemoryMappedRegisters::new(status, register1, register2));
    assert_eq!(mapped.status(), status);
    assert_eq!(mapped.register1(), register1);
    assert_eq!(mapped.register2(), register2);

    // Reserved fields are only getter-accessible for debug purposes.
    // This might change (i.e. no getter available, only debug-fmt info).
    // If you think you need them to be accessible, please contact us!
    assert_eq!(mapped.reserved_iii(), 0);
    assert_eq!(
        format!("{mapped:?}"),
        "MemoryMappedRegisters { reserved_i: 0, status: 3, register1: 32904, reserved_ii: 0, register2: 1008, reserved_iii: 0 }"
    );
}
