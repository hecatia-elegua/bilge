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
    for value in 0..4 {
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
            Err(e) => assert_eq!(format!("{e:?}"), "BitsError"),
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

#[bitsize(32)]
#[derive(DebugBits, FromBits)]
struct TupleStruct(u2, u6, u7, u8, u8, u1);

#[test]
fn tuple_struct() {
    let val_0 = u2::new(0);
    let val_1 = u6::new(1);
    let val_2 = u7::new(2);
    let val_3 = u8::new(3);
    let val_4 = u8::new(4);
    let val_5 = u1::new(0);
    let mut a = TupleStruct::new(val_0, val_1, val_2, val_3, val_4, val_5);
    assert_eq!(a.val_0(), val_0);
    assert_eq!(a.val_1(), val_1);
    assert_eq!(a.val_2(), val_2);
    assert_eq!(a.val_3(), val_3);
    assert_eq!(a.val_4(), val_4);
    assert_eq!(a.val_5(), val_5);
    let val_0 = u2::new(1);
    a.set_val_0(val_0);
    assert_eq!(a.val_0(), val_0);
    assert_eq!(a.val_1(), val_1);
    assert_eq!(a.val_2(), val_2);
    assert_eq!(a.val_3(), val_3);
    assert_eq!(a.val_4(), val_4);
    assert_eq!(a.val_5(), val_5);
    let val_3 = u8::MAX;
    a.set_val_3(val_3);
    assert_eq!(a.val_0(), val_0);
    assert_eq!(a.val_1(), val_1);
    assert_eq!(a.val_2(), val_2);
    assert_eq!(a.val_3(), val_3);
    assert_eq!(a.val_4(), val_4);
    assert_eq!(a.val_5(), val_5);
}

#[bitsize(57)]
#[derive(FromBits, DebugBits, PartialEq)]
struct Basic {
    arr: [u4; 12],
    tup: (bool, bool, bool),
    tup_arr: [(u2, bool); 2],
}

#[test]
fn other_field_types() {
    let mut basic = Basic::from(u57::new(0));

    // arrays can be accessed fully or per element, via index
    let sixth = u4::new(0b1111);
    let eleventh = u4::new(0b1101);
    basic.set_arr_at(6, sixth);
    basic.set_arr_at(11, eleventh);

    let z = u4::new(0);
    let mut arr = [z, z, z, z, z, z, sixth, z, z, z, z, eleventh];
    assert_eq!(basic.arr(), arr);
    assert_eq!(sixth, basic.arr_at(6));
    assert_eq!(eleventh, basic.arr_at(11));

    arr.reverse();
    basic.set_arr(arr);
    assert_eq!(basic.arr(), arr);
    assert_eq!(z, basic.arr_at(6));
    assert_eq!(z, basic.arr_at(11));
    assert_eq!(sixth, basic.arr_at(5));
    assert_eq!(eleventh, basic.arr_at(0));

    // tuples can only be accessed fully
    let z = (false, false, false);
    let tup = (false, true, true);
    assert_eq!(basic.tup(), z);
    basic.set_tup(tup);
    assert_eq!(basic.tup(), tup);

    // nesting - tuples in array
    let z = (u2::new(0), false);
    let zeroth = (u2::new(0b11), true);
    basic.set_tup_arr_at(0, zeroth);
    assert_eq!(basic.tup_arr(), [zeroth, z]);
    assert_eq!(zeroth, basic.tup_arr_at(0));
    assert_eq!(z, basic.tup_arr_at(1));

    let first = (u2::new(0b10), false);
    basic.set_tup_arr_at(1, first);
    assert_eq!(basic.tup_arr(), [zeroth, first]);
    assert_eq!(zeroth, basic.tup_arr_at(0));
    assert_eq!(first, basic.tup_arr_at(1));
}

#[bitsize(39)]
#[derive(FromBits, DebugBits, PartialEq)]
struct NestedMess {
    tu_tuple_ple: (u1, (u2, u8), u1),
    // this has special handling, transmuting [[]] to [] internally to generate less
    arr_arr_ay_ay: [[InnerTupleStruct; 2]; 2],
    bit: u1,
    arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay: [[([[(InnerTupleStruct, u2); 2]; 1], u1); 2]; 1],
}

#[bitsize(2)]
#[derive(Clone, Copy, FromBits, DebugBits, PartialEq)]
struct InnerTupleStruct(u1, bool);

#[bitsize(18)]
#[derive(TryFromBits, DebugBits, PartialEq)]
struct UnfilledEnumMess {
    big_fumble: [[([[(HaveFun, u2); 2]; 1], u1); 2]; 1],
}

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum HaveFun { Yes, No, Maybe, }

/// also see `examples/nested_tuples_and_arrays.rs`
#[test]
fn that_one_test() {
    let tu_tuple_ple = (u1::new(0), (u2::new(0b00), 0b1111_1111), u1::new(1));
    let arr_arr_ay_ay = [
        [InnerTupleStruct::from(u2::new(3)), InnerTupleStruct::from(u2::new(0b10))],
        [InnerTupleStruct::from(u2::new(3)), InnerTupleStruct::from(u2::new(0))]
    ];
    let bit = u1::new(1);
    let arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay = [[
        (
            [[(InnerTupleStruct::from(u2::new(3)), u2::new(3)), (InnerTupleStruct::from(u2::new(3)), u2::new(3))]],
            u1::new(0)
        ),
        (
            [[(InnerTupleStruct::from(u2::new(0b10)), u2::new(3)), (InnerTupleStruct::from(u2::new(3)), u2::new(3))]],
            u1::new(0)
        )
    ]];
    let mut mess = NestedMess::new(tu_tuple_ple, arr_arr_ay_ay, bit, arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay);
    // dbg!(&mess);
    assert_eq!(mess, NestedMess::from(u39::new(0b0_1_1111_110_0_1_1111_111__1_0011_1011__1__111_1111_1000)));
    assert_eq!(tu_tuple_ple, mess.tu_tuple_ple());
    assert_eq!(arr_arr_ay_ay, mess.arr_arr_ay_ay());
    assert_eq!(bit, mess.bit());
    assert_eq!(arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay, mess.arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay());
    
    let tu_tuple_ple = (u1::new(0), (u2::new(0b10), 0b1010_0100), u1::new(0));
    mess.set_tu_tuple_ple(tu_tuple_ple);
    assert_eq!(tu_tuple_ple, mess.tu_tuple_ple());
    assert_eq!(arr_arr_ay_ay, mess.arr_arr_ay_ay());
    assert_eq!(bit, mess.bit());
    assert_eq!(arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay, mess.arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay());

    let elem_0 = [InnerTupleStruct::from(u2::new(0)), InnerTupleStruct::from(u2::new(0b01))];
    let elem_1 = [InnerTupleStruct::from(u2::new(0)), InnerTupleStruct::from(u2::new(3))];
    let arr_arr_ay_ay = [elem_0, elem_1];
    mess.set_arr_arr_ay_ay(arr_arr_ay_ay);
    assert_eq!(tu_tuple_ple, mess.tu_tuple_ple());
    assert_eq!(arr_arr_ay_ay, mess.arr_arr_ay_ay());
    assert_eq!(bit, mess.bit());
    assert_eq!(arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay, mess.arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay());

    let arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay = [[
        (
            [[(InnerTupleStruct::from(u2::new(0)), u2::new(0)), (InnerTupleStruct::from(u2::new(0b01)), u2::new(0))]],
            u1::new(0)
        ),
        (
            [[(InnerTupleStruct::from(u2::new(0)), u2::new(1)), (InnerTupleStruct::from(u2::new(3)), u2::new(0))]],
            u1::new(0)
        )
    ]];
    mess.set_arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay(arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay);
    assert_eq!(tu_tuple_ple, mess.tu_tuple_ple());
    assert_eq!(arr_arr_ay_ay, mess.arr_arr_ay_ay());
    assert_eq!(bit, mess.bit());
    assert_eq!(arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay, mess.arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay());
    // dbg!(&mess);

    let uem1 = UnfilledEnumMess::try_from(u18::new(0b1_0101_1110_0_1010_1010)).unwrap();
    let uem2 = UnfilledEnumMess::new(
        [[
            (
                [[(HaveFun::Maybe, u2::new(2)), (HaveFun::Maybe, u2::new(2))]],
                u1::new(0)
            ),
            (
                [[(HaveFun::Maybe, u2::new(3)), (HaveFun::No, u2::new(1))]],
                u1::new(1)
            )
        ]]
    );
    assert_eq!(uem1.value, uem2.value);
    assert_eq!(uem1, uem2);
    let raw = u18::new(0b1_0101_11___11____0_1010_1010);
    let err = UnfilledEnumMess::try_from(raw);
    assert!(err.is_err());

    // mess.arr_arr_ay_ay_at(2); //panics, like it should

    assert_eq!(elem_0, mess.arr_arr_ay_ay_at(0));
    assert_eq!(elem_1, mess.arr_arr_ay_ay_at(1));
    mess.set_arr_arr_ay_ay_at(0, elem_1);
    mess.set_arr_arr_ay_ay_at(1, elem_0);
    assert_eq!(elem_1, mess.arr_arr_ay_ay_at(0));
    assert_eq!(elem_0, mess.arr_arr_ay_ay_at(1));
}

#[bitsize(16)]
#[derive(DebugBits)]
struct Array([u4; 4]);

#[test]
#[should_panic(expected = "assertion failed: index < 4")]
fn oob() {
    let zero = u4::new(0);
    let mut arrrr = Array::new([zero, zero, zero, zero]);
    let one = u4::new(1);
    arrrr.set_val_0_at(0, one);
    arrrr.set_val_0_at(1, one);
    assert_eq!(format!("{arrrr:?}"), "Array([1, 1, 0, 0])");
    arrrr.set_val_0_at(2, one);
    arrrr.set_val_0_at(3, one);
    assert_eq!(format!("{arrrr:?}"), "Array([1, 1, 1, 1])");
    // out of bounds
    arrrr.set_val_0_at(4, one);
}
#[test]
#[should_panic(expected = "assertion failed: index < 4")]
fn oob2() { Array::new([u4::new(0), u4::new(0), u4::new(0), u4::new(0)]).val_0_at(4); }

#[bitsize(8)]
#[derive(Default, PartialEq, DebugBits)]
struct NestedNonZeroDefault {
    field1: u2,
    field2: u4,
    field3: Cool,
}

#[bitsize(2)]
#[derive(Default, FromBits, PartialEq, Debug, Clone, Copy)]
enum Cool {
    Coool,
    Cooool,
    #[default]
    CooooolDefault,
    Cooooool,
}

#[bitsize(34)]
#[derive(DefaultBits, PartialEq, DebugBits, FromBits)]
struct ArrayTupleDefault {
    field1: [[((u2, Cool, bool), (bool, bool, Cool)); 2]; 1],
    field2: ([Cool; 2], [(u2, Cool); 3]),
}

#[test]
fn default_bits() {
    let default = NestedNonZeroDefault::default();
    assert_eq!(default, NestedNonZeroDefault::new(u2::new(0), u4::new(0), Cool::CooooolDefault));

    let default = ArrayTupleDefault::default();
    assert_eq!(default, ArrayTupleDefault::from(u34::new(0b1000_1000_1000_10_10_1000_01000_1000_01000)));
}
