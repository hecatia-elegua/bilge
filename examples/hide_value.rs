#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
use bilge::prelude::*;

#[bitsize(4)]
#[derive(TryFromBits, Debug, PartialEq, Clone, Copy)]
enum Status {
    Ready,
    Busy,
    Failed,
}

#[bitsize(8)]
#[derive(TryFromBits, DebugBits, PartialEq, Clone, Copy)]
struct Register {
    header: u4,
    body: Status,
}

#[bitsize(8, hide_value)]
#[derive(TryFromBits, DebugBits, PartialEq, Clone, Copy)]
struct HiddenRegister {
    header: u4,
    body: Status,
}

fn main() {
    let mut leaky = Register::new(u4::new(0xA), Status::Ready);
    // There is no `Status` for `3`, but we can still write it through `.value`:
    leaky.value = u8::new(0x3A);
    assert_eq!(leaky.value, u8::new(0x3A));
    // This panics internally:
    // leaky.body();

    let hidden = HiddenRegister::new(u4::new(0xA), Status::Ready);
    assert_eq!(hidden.header(), u4::new(0xA));
    // hidden.value = u8::new(0x3A); // does not compile

    // The whole value is still available through `TryFrom`/`From`:
    let raw = u8::from(hidden);
    let parsed = HiddenRegister::try_from(raw).unwrap();
    assert_eq!(hidden, parsed);
    assert!(HiddenRegister::try_from(u8::new(0x3A)).is_err());

    // Custom conversions can still pack a larger word by calling `new`.
    let sparse = SparseBits::from(0b1000_0000_0000_0000_0000_0000_0000_1100u32);
    println!("{sparse:?}");
}

/// Custom conversions can still pack a larger word by calling `new`.
#[bitsize(3, hide_value)]
#[derive(DebugBits, FromBits)]
struct SparseBits {
    one: bool,
    two: bool,
    three: bool,
}

impl From<u32> for SparseBits {
    fn from(value: u32) -> Self {
        // bits 2, 3 and 31 of a larger word
        Self::new((value >> 2 & 1) != 0, (value >> 3 & 1) != 0, (value >> 31 & 1) != 0)
    }
}
