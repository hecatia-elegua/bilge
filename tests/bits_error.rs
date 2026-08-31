#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl))]
use bilge::prelude::*;

#[bitsize(4)]
#[derive(TryFromBits, Debug, PartialEq)]
enum Foo {
    One = 0b0001,
    Two = 0b0010,
}

#[bitsize(4)]
#[derive(TryFromBits, Debug, PartialEq)]
enum Bar {
    Three = 0b0011,
    Four = 0b0100,
}

#[bitsize(8)]
#[derive(TryFromBits, DebugBits)]
struct Byte {
    foo: Foo,
    bar: Bar,
}

#[bitsize(8)]
#[derive(TryFromBits, DebugBits)]
struct Nested {
    inner: Byte,
}

#[bitsize(8)]
#[derive(TryFromBits, DebugBits)]
struct Outer {
    wrapper: Nested,
}

#[bitsize(4)]
#[derive(TryFromBits, DebugBits)]
struct Pair([HaveFun; 2]);

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum HaveFun {
    Yes,
    No,
    Maybe,
}

#[test]
fn names_the_enum_that_has_no_matching_variant() {
    // foo = 0b0001 (Foo::One), bar = 0b0001 (no Bar variant)
    let err = Byte::try_from(u8::new(0b0001_0001)).unwrap_err();
    assert_eq!(err.type_name(), "Bar");
    assert_eq!(err.field_name(), Some("bar"));
    assert_eq!(err.invalid_bits(), 0b0001);
    assert_eq!(err.bitsize(), 4);
    assert_eq!(err.bit_start(), 4);
    assert_eq!(err.bit_end(), 7);
    assert_eq!(format!("{err}"), "`Bar` has no representation for 0b0001 (field `bar`, bits 4..=7)");
}

#[test]
fn reports_the_first_failing_field() {
    // foo = 0b0000 (no Foo variant), conversion stops here, even though bar is also invalid
    let err = Byte::try_from(u8::new(0b0001_0000)).unwrap_err();
    assert_eq!(err.type_name(), "Foo");
    assert_eq!(err.field_name(), Some("foo"));
    assert_eq!(err.invalid_bits(), 0);
    assert_eq!(err.bitsize(), 4);
    assert_eq!(err.bit_start(), 0);
    assert_eq!(err.bit_end(), 3);
    assert_eq!(format!("{err}"), "`Foo` has no representation for 0b0000 (field `foo`, bits 0..=3)");
}

#[test]
fn nested_struct_names_the_innermost_enum() {
    let err = Nested::try_from(u8::new(0b0001_0001)).unwrap_err();
    assert_eq!(err.type_name(), "Bar");
    assert_eq!(err.field_name(), Some("bar"));
    assert_eq!(err.field_path(), &["inner", "bar"]);
    assert_eq!(err.invalid_bits(), 0b0001);
    assert_eq!(err.bitsize(), 4);
    assert_eq!(err.bit_start(), 4);
    assert_eq!(err.bit_end(), 7);
    assert_eq!(format!("{err}"), "`Bar` has no representation for 0b0001 (field `inner.bar`, bits 4..=7)");

    let err = Outer::try_from(u8::new(0b0001_0001)).unwrap_err();
    assert_eq!(err.field_path(), &["wrapper", "inner", "bar"]);
    assert_eq!(
        format!("{err}"),
        "`Bar` has no representation for 0b0001 (field `wrapper.inner.bar`, bits 4..=7)"
    );
}

#[test]
fn array_element_offset() {
    // second HaveFun = 3, first is Yes=0
    let err = Pair::try_from(u4::new(0b11_00)).unwrap_err();
    assert_eq!(err.type_name(), "HaveFun");
    assert_eq!(err.field_name(), Some("0"));
    assert_eq!(err.invalid_bits(), 3);
    assert_eq!(err.bitsize(), 2);
    assert_eq!(err.bit_start(), 2);
    assert_eq!(err.bit_end(), 3);
    assert_eq!(format!("{err}"), "`HaveFun` has no representation for 0b11 (field `0`, bits 2..=3)");
}

#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D1 {
    e: HaveFun,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D2 {
    d1: D1,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D3 {
    d2: D2,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D4 {
    d3: D3,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D5 {
    d4: D4,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D6 {
    d5: D5,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D7 {
    d6: D6,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D8 {
    d7: D7,
}
#[bitsize(2)]
#[derive(TryFromBits, DebugBits)]
struct D9 {
    d8: D8,
}

#[test]
fn field_path_keeps_eight_nested_names() {
    let err = D9::try_from(u2::new(3)).unwrap_err();
    assert_eq!(err.type_name(), "HaveFun");
    // outermost name is dropped; bits still point at the field
    assert_eq!(err.field_path(), &["d7", "d6", "d5", "d4", "d3", "d2", "d1", "e"]);
    assert_eq!(err.bit_start(), 0);
    assert_eq!(err.bit_end(), 1);
}

#[bitsize(4)]
#[derive(TryFromBits, DebugBits)]
struct NamedArray {
    vals: [HaveFun; 2],
}

#[test]
fn array_uses_bit_range_not_an_index() {
    let err = NamedArray::try_from(u4::new(0b11_00)).unwrap_err();
    assert_eq!(err.field_path(), &["vals"]);
    assert_eq!(err.bit_start(), 2);
    assert_eq!(err.bit_end(), 3);
    assert_eq!(format!("{err}"), "`HaveFun` has no representation for 0b11 (field `vals`, bits 2..=3)");
}

#[bitsize(4)]
#[derive(TryFromBits, DebugBits)]
struct TupleField {
    data: (HaveFun, HaveFun),
}

#[test]
fn tuple_element_is_a_path_index() {
    let err = TupleField::try_from(u4::new(0b11_00)).unwrap_err();
    assert_eq!(err.field_path(), &["data", "1"]);
    assert_eq!(err.bit_start(), 2);
    assert_eq!(err.bit_end(), 3);
    assert_eq!(format!("{err}"), "`HaveFun` has no representation for 0b11 (field `data.1`, bits 2..=3)");
}
