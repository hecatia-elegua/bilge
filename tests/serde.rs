#![cfg(feature = "serde")]
#![allow(clippy::unusual_byte_groupings)]

use bilge::prelude::*;
use serde_test::{Configure, Readable, Token, assert_de_tokens, assert_de_tokens_error, assert_ser_tokens, assert_tokens};

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
        &bits.readable(),
        &[
            Token::Map { len: Some(2) },
            Token::Str("field1"),
            Token::U8(0b00100011),
            Token::Str("field2"),
            Token::U8(0b01001),
            Token::MapEnd,
        ],
    );
}

#[test]
fn serde_struct_compact() {
    let bits = BitsStruct::from(u17::new(0b0_01001_0_00100011_0_0));

    assert_tokens(
        &bits.compact(),
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
    assert_de_tokens_error::<Readable<BitsStruct>>(
        &[Token::Map { len: Some(1) }, Token::Str("field1"), Token::U8(0b00100011), Token::MapEnd],
        "missing field `field2`",
    );
}

#[test]
fn serde_struct_extra_field() {
    assert_de_tokens_error::<Readable<BitsStruct>>(
        &[
            Token::Map { len: Some(3) },
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
        &[
            Token::TupleStruct {
                name: "BitsTupleStruct",
                len: 3,
            },
            Token::Str("val_0"),
        ],
        r#"invalid type: string "val_0", expected u8"#,
    );
}

#[bitsize(17)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits)]
struct BitsStructSigned {
    padding: u1,
    reserved: i1,
    field1: i8,
    padding: u1,
    field2: u5,
    reserved: u1,
}

#[test]
fn serde_struct_signed() {
    let bits = BitsStructSigned::from(u17::new(0b0_01001_0_00100011_0_0));

    assert_tokens(
        &bits.readable(),
        &[
            Token::Map { len: Some(2) },
            Token::Str("field1"),
            Token::I8(0b00100011),
            Token::Str("field2"),
            Token::U8(0b01001),
            Token::MapEnd,
        ],
    );
}

#[test]
fn serde_struct_missing_field_signed() {
    assert_de_tokens_error::<Readable<BitsStructSigned>>(
        &[Token::Map { len: Some(1) }, Token::Str("field1"), Token::U8(0b00100011), Token::MapEnd],
        "missing field `field2`",
    );
}

#[test]
fn serde_struct_extra_field_signed() {
    assert_de_tokens_error::<Readable<BitsStructSigned>>(
        &[
            Token::Map { len: Some(3) },
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
struct BitsTupleStructSigned(u8, i5);

#[test]
fn serde_tuple_struct_signed() {
    let bits = BitsTupleStructSigned::from(u13::new(0b01001_00100011));

    assert_tokens(
        &bits,
        &[
            Token::TupleStruct {
                name: "BitsTupleStructSigned",
                len: 2,
            },
            Token::U8(0b00100011),
            Token::I8(0b01001),
            Token::TupleStructEnd,
        ],
    );
}

#[test]
fn serde_tuple_struct_map_signed() {
    assert_de_tokens_error::<BitsTupleStructSigned>(
        &[
            Token::TupleStruct {
                name: "BitsTupleStructSigned",
                len: 3,
            },
            Token::Str("val_0"),
        ],
        r#"invalid type: string "val_0", expected u8"#,
    );
}

#[bitsize(16)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits, Clone, Copy)]
struct StatusAt {
    #[at(3)]
    ready: bool,
    #[at(8..=11)]
    nack: u4,
}

#[test]
fn serde_at_holes_are_like_reserved() {
    let from_new = StatusAt::new(true, u4::new(0xA));
    assert_tokens(
        &from_new.readable(),
        &[
            Token::Map { len: Some(2) },
            Token::Str("ready"),
            Token::Bool(true),
            Token::Str("nack"),
            Token::U8(0xA),
            Token::MapEnd,
        ],
    );

    // leftover hole bits are not in the data, same as reserved/padding
    let from_raw = StatusAt::from(u16::new(0xFFFF));
    let named_only = StatusAt::new(true, u4::new(0xF));
    assert_ne!(from_raw, named_only);
    assert_ser_tokens(
        &from_raw.readable(),
        &[
            Token::Map { len: Some(2) },
            Token::Str("ready"),
            Token::Bool(true),
            Token::Str("nack"),
            Token::U8(0xF),
            Token::MapEnd,
        ],
    );
}

#[bitsize(8)]
#[derive(FromBits, PartialEq, SerializeBits, DeserializeBits, DebugBits, Clone, Copy)]
struct WithDefault {
    a: u4,
    #[default(u4::new(0xF))]
    b: u4,
}

#[test]
fn serde_default_field_is_optional_like_serde_default() {
    let with_default_b = WithDefault::new(u4::new(1), u4::new(0xF));
    assert_de_tokens(
        &with_default_b.readable(),
        &[Token::Map { len: Some(1) }, Token::Str("a"), Token::U8(1), Token::MapEnd],
    );
    assert_de_tokens(
        &with_default_b.compact(),
        &[
            Token::Struct { name: "WithDefault", len: 1 },
            Token::Str("a"),
            Token::U8(1),
            Token::StructEnd,
        ],
    );

    let overridden = WithDefault::new(u4::new(1), u4::new(2));
    assert_tokens(
        &overridden.readable(),
        &[
            Token::Map { len: Some(2) },
            Token::Str("a"),
            Token::U8(1),
            Token::Str("b"),
            Token::U8(2),
            Token::MapEnd,
        ],
    );
}
