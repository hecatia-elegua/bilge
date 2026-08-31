//! Generated code must compile without `bilge::prelude`.
use bilge::arbitrary_int::{u2, u4};
use bilge::{BinaryBits, BuilderBits, DebugBits, DefaultBits, FromBits, TryFromBits, bitsize};

#[bitsize(8, new = pub)]
#[derive(FromBits, DebugBits, BinaryBits, DefaultBits, BuilderBits)]
struct Packed {
    a: u4,
    b: u4,
}

#[bitsize(2)]
#[derive(TryFromBits, BinaryBits)]
enum Flag {
    Off,
    On,
    Unknown,
}

#[bitsize(4)]
#[discriminant_at(0..=1)]
#[derive(FromBits, BinaryBits)]
enum Tagged {
    Lo(u2) = 0,
    Hi(u2) = 1,
    A(u2) = 2,
    B(u2) = 3,
}

#[test]
fn compiles_and_works_without_prelude() {
    let p = Packed::new(u4::new(1), u4::new(2));
    assert_eq!(format!("{p:b}"), "0010_0001");
    assert_eq!(u8::from(p), 0b0010_0001);
    let _ = Packed::default();
    let built = Packed::builder().a(u4::new(3)).b(u4::new(4)).build();
    assert_eq!(built.a(), u4::new(3));

    assert!(Flag::try_from(u2::new(0)).is_ok());
    let tagged = Tagged::Hi(u2::new(0b01));
    assert_eq!(u4::from(tagged).value(), 0b01_01);
}
