#![cfg(feature = "serde")]

use bilge::prelude::*;
use serde_test::{assert_de_tokens_error, assert_tokens, Token};

#[bitsize(17)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits)]
struct BitsStruct {
    field1: u8,
    field2: u8,
    padding: u1,
}

#[test]
fn serde_struct() {
    let bits = BitsStruct::from(u17::new(0b0_0000_0001_0010_0011));

    assert_tokens(
        &bits,
        &[
            Token::Struct { name: "BitsStruct", len: 2 },
            Token::Str("field1"),
            Token::U8(0b0010_0011),
            Token::Str("field2"),
            Token::U8(0b0000_0001),
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
            Token::U8(0b0010_0011),
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
            Token::U8(0b0010_0011),
            Token::Str("field2"),
            Token::U8(0b0000_0001),
            Token::Str("field3"),
        ],
        "unknown field `field3`, expected `field1` or `field2`",
    );
}

#[bitsize(16)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits)]
struct BitsTupleStruct(u8, u8);

#[test]
fn serde_tuple_struct() {
    let bits = BitsTupleStruct::from(0b0000_0001_0010_0011);

    assert_tokens(
        &bits,
        &[
            Token::TupleStruct {
                name: "BitsTupleStruct",
                len: 2,
            },
            Token::U8(0b0010_0011),
            Token::U8(0b0000_0001),
            Token::TupleStructEnd,
        ],
    );
}

#[test]
fn serde_tuple_struct_map() {
    assert_de_tokens_error::<BitsTupleStruct>(
        &[
            Token::TupleStruct { name: "BitsStruct", len: 3 },
            Token::Str("val_0"),
        ],
        r#"invalid type: string "val_0", expected u8"#,
    );
}
