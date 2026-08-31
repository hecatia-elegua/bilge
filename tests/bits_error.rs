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
    assert_eq!(err.field_name(), Some("val_0"));
    assert_eq!(err.invalid_bits(), 3);
    assert_eq!(err.bitsize(), 2);
    assert_eq!(err.bit_start(), 2);
    assert_eq!(err.bit_end(), 3);
    assert_eq!(err.array_index(), Some(1));
    assert_eq!(format!("{err}"), "`HaveFun` has no representation for 0b11 (field `val_0[1]`, bits 2..=3)");
}
