#![cfg(feature = "serde")]
#![allow(clippy::unusual_byte_groupings)]

use bilge::prelude::*;
use serde_test::{assert_de_tokens_error, assert_tokens, Token};

#[bitsize(17)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits)]
struct BitsStruct {
    padding: u1,
    reserved: u1,
    field1: u8,
    padding: u1,
    field2: u5,
    reserved: u1,
}

#[test]
fn serde_struct() {
    let bits = BitsStruct::from(u17::new(0b0_01001_0_00100011_0_0));

    assert_tokens(
        &bits,
        &[
            Token::Struct { name: "BitsStruct", len: 2 },
            Token::Str("field1"),
            Token::U8(0b00100011),
            Token::Str("field2"),
            Token::U8(0b01001),
            Token::StructEnd,
        ],
    );
}

#[test]
fn serde_struct_missing_field() {
    assert_de_tokens_error::<BitsStruct>(
        &[
            Token::Struct { name: "BitsStruct", len: 1 },
            Token::Str("field1"),
            Token::U8(0b00100011),
            Token::StructEnd,
        ],
        "missing field `field2`",
    );
}

#[test]
fn serde_struct_extra_field() {
    assert_de_tokens_error::<BitsStruct>(
        &[
            Token::Struct { name: "BitsStruct", len: 3 },
            Token::Str("field1"),
            Token::U8(0b00100011),
            Token::Str("field2"),
            Token::U8(0b01001),
            Token::Str("field3"),
        ],
        "unknown field `field3`, expected `field1` or `field2`",
    );
}

#[bitsize(13)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits)]
struct BitsTupleStruct(u8, u5);

#[test]
fn serde_tuple_struct() {
    let bits = BitsTupleStruct::from(u13::new(0b01001_00100011));

    assert_tokens(
        &bits,
        &[
            Token::TupleStruct {
                name: "BitsTupleStruct",
                len: 2,
            },
            Token::U8(0b00100011),
            Token::U8(0b01001),
            Token::TupleStructEnd,
        ],
    );
}

#[test]
fn serde_tuple_struct_map() {
    assert_de_tokens_error::<BitsTupleStruct>(
        &[Token::TupleStruct { name: "BitsStruct", len: 3 }, Token::Str("val_0")],
        r#"invalid type: string "val_0", expected u8"#,
    );
}
