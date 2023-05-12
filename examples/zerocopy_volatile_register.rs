#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs, const_maybe_uninit_write))]
use bilge::prelude::*;
use volatile::{Volatile, ReadOnly};
use zerocopy::FromBytes;

#[derive(Debug, FromBytes)]
struct Redistributor {
    control: Volatile<RedistributorControl>,
    // this is just an example, not how the real GIC is structured
    group: ReadOnly<Group>,
}

#[bitsize(32)]
// we only want this to be FromBytes if it is also FromBits, FromBytes just acts on the final bitstruct (so, on a u32)
#[derive(Copy, Clone, DebugBits, FromBits, FromBytes)]
struct RedistributorControl {
    // padding is currently handled like reserved
    padding: u2,
    pub three: bool,
    // visibility works, though setter and getter have the same visibility, like with usual rust struct field access
    pub(crate) four: bool,
    // reserved without numbers
    reserved: u20,
    five: bool,
    six: bool,
    seven: bool,
    // reserved without numbers
    reserved: u4,
    uwp: bool,
}

// generating these would be nice to have
impl core::fmt::Binary for RedistributorControl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let val = self.value;
        core::fmt::Binary::fmt(&val, f)
    }
}

#[bitsize(32)]
#[derive(Clone, Copy, DebugBits, FromBits, FromBytes)]
struct Group([bool; 32]);

impl core::fmt::Binary for Group {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let val = self.value;
        core::fmt::Binary::fmt(&val, f)
    }
}

fn main() {
    // let raw_memory: &[u8] = &[0u8, 1, 2, 3, 255, 255, 254, 255];
    // The latest version of zerocopy does this, but in our case we use an older version.
    // let redist = LPIRedistributor::read_from(raw_memory).unwrap();

    let raw_memory = [0u8, 1, 2, 3, 255, 255, 254, 255];
    let redist: Redistributor = unsafe { core::mem::transmute(raw_memory) };
    println!("{:032b}", redist.control.read());
    println!("{:?}", redist.control);
    println!("{:032b}", redist.group.read());
    println!("{:?}", redist.group);
}
