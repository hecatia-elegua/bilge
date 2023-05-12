#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
#![allow(clippy::unusual_byte_groupings)]
use bilge::prelude::*;

#[bitsize(4)]
#[derive(TryFromBits)]
struct CanBeChanged(Unfilled);

#[bitsize(4)]
#[derive(TryFromBits)]
enum Unfilled {
    A, B, C,
}

fn main() {
    // This file mostly shows one flaw to still be solved or at least to be made configurable:
    // The inner value of a bitfield, which holds invariants, can currently still be changed.
    let mut a = CanBeChanged::new(Unfilled::A);
    // There is no enum value for `3` or `0b11`, but we can set it anyways:
    a.value = u4::new(3);
    // This panics internally:
    // a.val_0();

    // Here we try to use a custom impl and also put the generated code inside a module,
    // thereby making `.value` inaccessible.
    // Let's say we want bit 3, 4 and 31, so 0, 1, 1:
    let value = 0b11111111_00001111_00110011_10101010;
    let thing = SomeBits::from(value);
    println!("{thing:?}");
}

mod somebits {
    use super::*;
    #[bitsize(3)]
    #[derive(DebugBits)]
    pub struct SomeBits {
        one: bool,
        two: bool,
        three: bool,
    }

    // should not really be allowed, though might be useful for optimization
    // we could provide a feature toggle, or just an attribute param to control this
    //
    // basically, putting the generated struct ThingsIWant into a module and exposing
    // it with a use statement again would be less "leaky", but this would also
    // mean all the private items would not be accessible..
    //
    // If we find any other way to make `value` inaccessible, update this.
    #[allow(dead_code)]
    fn modify_inner() {
        let a = 0b10101010;
        let mut b = SomeBits::new(true, true, true);
        b.value = u3::new(a);
    }
}
use somebits::*;

impl SomeBits {
    #[allow(dead_code)]
    fn modify_outside() {
        let b = SomeBits::new(true, true, true);
        // b.one(); //this is private
        println!("can't do anything with b in here! {b:?}");
    }
}

// custom impl, which might be useful in some cases
impl From<u32> for SomeBits {
    fn from(value: u32) -> Self {
        //let's say we want bit 3, 4 and 31
        let one_two = value >> 2 & (<u2 as Bitsized>::MAX.value() as u32);
        let three = value >> 30 & <bool as Bitsized>::MAX.value() as u32;
        let value = u3::new((one_two | three << 2) as u8);
        // If value was a public field, this would work.
        // Self { value }
        // Else, only Self::new(one, two, three) works, which is good enough?
        let one = u1::new(value.value() & <u1 as Bitsized>::MAX.value()).into();
        let two = u1::new(value.value() >> 1 & <u1 as Bitsized>::MAX.value()).into();
        let three = u1::new(value.value() >> 2 & <u1 as Bitsized>::MAX.value()).into();
        Self::new(one, two, three)
    }
}
