use bilge::prelude::*;
use custom_bits::FieldsInBits;

#[test]
fn custom_derive() {
    #[bitsize(6)]
    #[derive(FieldsInBits, DebugBits, FromBits)]
    struct Basic {
        field_1: u3,
        field_2: u3,
    }
    assert_eq!(Basic::field_count(), 2);
}
