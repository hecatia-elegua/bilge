use bilge::prelude::*;
use custom_bits::CustomBits;

#[test]
fn custom_derive() {
    #[bitsize(6)]
    #[derive(CustomBits, DebugBits, FromBits)]
    struct Basic {
        field_1: u3,
        field_2: u3,
    }
    let a = Basic::from(u6::new(0b001011));
    assert_eq!(a.fields(), 2);
}