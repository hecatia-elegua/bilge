//! Example of nested structs and enums.
//! Also tests documentation warnings are working right.
#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
#![deny(missing_docs)]
use bilge::prelude::*;

//it is still a little annoying that rust gives us the helpful message to implement "Debug" instead of "DebugBits"
/// This is ParentStruct.
#[bitsize(6)]
#[derive(DebugBits, TryFromBits)]
pub struct ParentStruct {
    /// This is field1.
    pub field1: ChildStruct,
    field2: ChildEnum,
    field3: u2,
}

/// This is ChildEnum.
#[bitsize(2)]
#[derive(Debug, FromBits)]
pub enum ChildEnum {
    /// This is A.
    A = 0b000,
    /// This is B.
    B = 0x001,
    /// This is C.
    C,
    /// This is D.
    D = 0o003,
}

#[bitsize(2)]
#[derive(DebugBits, TryFromBits)]
struct ChildStruct {
    field: NestedChildEnum,
}

#[bitsize(2)]
#[derive(Debug, TryFromBits)]
enum NestedChildEnum {
    A,
    // 1 is not defined
    B = 2,
    C, //also, "discriminant value `...` assigned more than once" is handled by rustc
}

fn main() {
    let mut a = ParentStruct::try_from(u6::new(0b001010)).unwrap();
    // ParentStruct { field1: ChildStruct { field: B }, field2: C, field3: 0 }
    // ChildStruct { field: B }
    // C
    // 10
    println!("{a:?}");
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.value);

    a.set_field1(ChildStruct::try_from(u2::new(0b00)).unwrap());
    // ChildStruct { field: A }
    // C
    // ParentStruct { field1: ChildStruct { field: A }, field2: C, field3: 0 }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    a.set_field2(ChildEnum::from(u2::new(0b01)));
    // ChildStruct { field: A }
    // B
    // ParentStruct { field1: ChildStruct { field: A }, field2: B, field3: 0 }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    println!();
    a.set_field3(u2::new(0b11));
    // ChildStruct { field: A }
    // B
    // 3
    // ParentStruct { field1: ChildStruct { field: A }, field2: B, field3: 3 }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.field3());
    println!("{a:?}");

    /////////////
    println!();
    println!();
    println!();
    let mut a = ParentStruct::new(ChildStruct::new(NestedChildEnum::A), ChildEnum::D, u2::new(0b00));
    // ParentStruct { field1: ChildStruct { field: A }, field2: D, field3: 0 }
    // ChildStruct { field: A }
    // D
    // 12
    println!("{a:?}");
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.value);

    a.set_field1(ChildStruct::try_from(u2::new(0b00)).unwrap());
    // ChildStruct { field: A }
    // D
    // ParentStruct { field1: ChildStruct { field: A }, field2: D, field3: 0 }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    a.set_field2(ChildEnum::from(u2::new(0b01)));
    // ChildStruct { field: A }
    // B
    // ParentStruct { field1: ChildStruct { field: A }, field2: B, field3: 0 }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{a:?}");

    println!();
    a.set_field3(u2::new(0b11));
    // ChildStruct { field: A }
    // B
    // 3
    // ParentStruct { field1: ChildStruct { field: A }, field2: B, field3: 3 }
    println!("{:?}", a.field1());
    println!("{:?}", a.field2());
    println!("{:?}", a.field3());
    println!("{a:?}");
}
