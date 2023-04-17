#![feature(const_convert, const_trait_impl, const_mut_refs)]
use arbitrary_int::{u2, u6, Number};
use bilge::{bitsize, FromBits, TryFromBits, DebugBits, Bitsized};

//it is still a little annoying that rust gives us the helpful message to implement "Debug" instead of "DebugBits"
#[bitsize(6)]
#[derive(DebugBits, TryFromBits)]
struct ParentStruct {
    field1: ChildStruct,
    field2: ChildEnum,
    field3: u2,
}

#[bitsize(2)]
#[derive(Debug, FromBits)]
enum ChildEnum {
    A = 0b000, B = 0x001, C, D = 0o003
}

#[bitsize(2)]
#[derive(DebugBits, TryFromBits)]
struct ChildStruct {
    field: NestedChildEnum,
}

#[bitsize(2)]
#[derive(Debug, TryFromBits)]
enum NestedChildEnum {
    A, B = 2, C, //also, "discriminant value `...` assigned more than once" is handled by rustc
}

fn main() {
    let mut a = ParentStruct::try_from(u6::new(0b001010)).unwrap();
    println!("{a:?}");
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.value);

    a.set_field1(ChildStruct::try_from(u2::new(0b00)).unwrap());
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    a.set_field2(ChildEnum::from(u2::new(0b01)));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    println!();
    a.set_field3(u2::new(0b11));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.field3());
    println!("{a:?}");

    /////////////
    println!();
    println!();
    println!();
    let mut a = ParentStruct::new(
        ChildStruct::new(NestedChildEnum::A),
        ChildEnum::D,
        u2::new(0b00),
    );
    println!("{a:?}");
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.value);

    a.set_field1(ChildStruct::try_from(u2::new(0b00)).unwrap());
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    a.set_field2(ChildEnum::from(u2::new(0b01)));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    println!();
    a.set_field3(u2::new(0b11));
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.field3());
    println!("{a:?}");
}
