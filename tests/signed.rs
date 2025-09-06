#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs, const_maybe_uninit_write))]
use bilge::prelude::*;

#[bitsize(27)]
#[derive(FromBits, PartialEq, DebugBits)]
struct BitStructSigned {
    x: i20,
    y: i7,
}

#[test]
fn bit_struct_signed() {
    let mut bits = BitStructSigned::from(u27::new(0b1010110_00110001000101000001));
    let new = BitStructSigned::new(i20::new(201025), i7::new(-42));
    let x = i20::new(201025);
    let y = i7::new(-42);

    assert_eq!(bits, new);
    assert_eq!(bits.x(), x);
    assert_eq!(bits.y(), y);

    bits.set_y(i7::new(0b0101010));
    let y = i7::new(42);
    assert_eq!(bits.y(), y);
}
