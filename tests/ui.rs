use bilge::prelude::*;
use custom_bits::CustomBits;
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

#[test]
fn ui_custom() {
    #[bitsize(6)]
    #[derive(CustomBits, DebugBits, FromBits)]
    struct Basic {
        field_1: u3,
        field_2: u3,
    }
    let a = Basic::from(u6::new(0b001011));
    assert_eq!(a.fields(), 2);
}
