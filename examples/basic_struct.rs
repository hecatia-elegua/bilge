#![feature(const_convert, const_trait_impl, const_mut_refs)]
use bilge::{bitsize, TryFromBits, FromBits, DebugBits, Bitsized, u2, u4, Number};

#[bitsize(4)]
#[derive(DebugBits, TryFromBits)]
struct IncompleteStruct {
    field1: u2,
    field2: CompleteEnum,
}
#[bitsize(2)]
#[derive(Debug, FromBits)]
enum CompleteEnum {
    A = 0, B = 1, C = 0b0000_0010, D = 3
}

fn main() {
    let mut a = IncompleteStruct::try_from(u4::new(0b1011)).unwrap();
    println!("{a:?}");
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.value);

    a.set_field1(u2::new(0b00));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    a.set_field2(CompleteEnum::from(u2::new(0b01)));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");
}
