#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs, const_maybe_uninit_write))]
#![allow(clippy::unusual_byte_groupings)]
use bilge::prelude::*;

#[bitsize(14)]
#[derive(FromBits)]
struct Register {
    header: u4,
    body: u7,
    footer: Footer,
}

#[bitsize(3)]
#[derive(FromBits)]
struct Footer {
    is_last: bool,
    code: Code,
}

#[bitsize(2)]
#[derive(FromBits)]
enum Code { Success, Error, IoError, GoodExample }

#[bitsize(2)]
#[derive(TryFromBits, Debug, PartialEq)]
enum Class {
    Mobile, Semimobile, Stationary = 0x3
}

#[bitsize(8)]
#[derive(TryFromBits, DebugBits)]
struct Device {
    reserved: u2,
    class: Class,
    reserved: u4,
}

#[bitsize(32)]
#[derive(FromBits)]
struct InterruptSetEnables([bool; 32]);

#[bitsize(32)]
#[derive(FromBits, Debug, PartialEq)]
enum Subclass {
    Mouse,
    Keyboard,
    Speakers,
    #[fallback]
    Reserved,
}

fn main() {
    let reg1 = Register::new(
        u4::new(0b1010),
        u7::new(0b010_1010),
        Footer::new(true, Code::GoodExample)
    );
    let mut reg2 = Register::from(u14::new(0b11_1_0101010_1010));
    assert_eq!(reg1.value, reg2.value);
    let _header = reg2.header();
    reg2.set_footer(Footer::new(false, Code::Success));

    let mut ise = InterruptSetEnables::from(0b0000_0000_0000_0000_0000_0000_0001_0000);
    let ise5 = ise.val_0_at(4);
    ise.set_val_0_at(2, ise5);
    assert_eq!(0b0000_0000_0000_0000_0000_0000_0001_0100, ise.value);

    assert_eq!(Subclass::Reserved, Subclass::from(3));
    let subclass = Subclass::from(42);
    let num = u32::from(subclass);
    assert_ne!(42, num);
    assert_eq!(3, num);

    let class = Class::try_from(u2::new(2));
    assert_eq!(class, Err(u2::new(2)));
    println!("{:?}", Device::try_from(0b0000_11_00));
    println!("{:?}", Device::new(Class::Mobile));
}
