#![cfg(feature = "serde")]
use bilge::prelude::*;

// defmt is intended to be used with macro_use to fully override all core formatting.
// We're checking that our macros don't use any unqualified macros shadowed by defmt (e.g. write!())
// in proc macros where we don't intend to interact with the target's stdout.
#[allow(unused)]
#[macro_use]
extern crate defmt;

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
enum HaveFun {
    Yes,
    No,
    Maybe,
}

/// Passes if it compiles, i.e. none of the derive macros are accidentally shadowed.
#[test]
fn nested_mess_compiles() {
    let tu_tuple_ple = (u1::new(0), (u2::new(0b00), 0b1111_1111), u1::new(1));
    let arr_arr_ay_ay = [
        [InnerTupleStruct::from(u2::new(3)), InnerTupleStruct::from(u2::new(0b10))],
        [InnerTupleStruct::from(u2::new(3)), InnerTupleStruct::from(u2::new(0))],
    ];
    let bit = u1::new(1);
    let arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay = [[
        (
            [[
                (InnerTupleStruct::from(u2::new(3)), u2::new(3)),
                (InnerTupleStruct::from(u2::new(3)), u2::new(3)),
            ]],
            u1::new(0),
        ),
        (
            [[
                (InnerTupleStruct::from(u2::new(0b10)), u2::new(3)),
                (InnerTupleStruct::from(u2::new(3)), u2::new(3)),
            ]],
            u1::new(0),
        ),
    ]];
    let _mess = NestedMess::new(tu_tuple_ple, arr_arr_ay_ay, bit, arr_arr_tu_arr_arr_tuple_ay_ay_ple_ay_ay);
}

#[bitsize(31)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits)]
struct BitsStruct {
    padding: u1,
    reserved: u1,
    field1: u8,
    padding: u1,
    field2: u5,
    reserved: u1,
    field3: i8,
    padding: u1,
    field4: i5,
}

/// Passes if it compiles. Checks serde-related macros.
#[test]
fn serde_struct_compiles() {
    let _serde_struct = BitsStruct::from(u31::new(0b01110_0_10110001_0_01001_0_00100011_0_0));
}
