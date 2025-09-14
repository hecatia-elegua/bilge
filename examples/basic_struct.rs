#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
use bilge::prelude::*;

#[bitsize(4)]
#[derive(DebugBits, TryFromBits)]
struct IncompleteStruct {
    field1: u2,
    field2: CompleteEnum,
}
#[bitsize(2)]
#[derive(Debug, FromBits)]
enum CompleteEnum {
    A = 0,
    B = 1,
    C = 0b0000_0010,
    D = 3,
}

fn main() {
    let mut a = IncompleteStruct::try_from(u4::new(0b1011)).unwrap();
    // IncompleteStruct { field1: 3, field2: C }
    // 3
    // C
    // 11
    println!("{a:?}");
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.value);

    a.set_field1(u2::new(0b00));
    // 0
    // C
    // IncompleteStruct { field1: 0, field2: C }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    // 0
    // B
    // IncompleteStruct { field1: 0, field2: B }
    a.set_field2(CompleteEnum::from(u2::new(0b01)));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");
}
