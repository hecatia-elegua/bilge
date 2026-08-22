#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs, const_maybe_uninit_write))]
use core::ptr::NonNull;

use bilge::prelude::*;
use volatile::{VolatilePtr, access::ReadOnly};
use zerocopy::{FromBytes, Immutable, KnownLayout};

// NOTE: Once upon a time, this was
// `Volatile<RedistributorControl>,`
// `ReadOnly<Group>,`
// and you could just `core::mem::transmute`,
// but this apparently can't just work.
// Read more about it in the `volatile` crate and repo.

#[derive(Debug)]
struct Redistributor<'a> {
    control: VolatilePtr<'a, RedistributorControl>,
    // this is just an example, not how the real GIC is structured
    group: VolatilePtr<'a, Group, ReadOnly>,
}

#[bitsize(32)]
// we only want this to be FromBytes if it is also FromBits, FromBytes just acts on the final bitstruct (so, on a u32)
#[derive(Copy, Clone, DebugBits, FromBits, BinaryBits, FromBytes, Immutable, KnownLayout)]
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

#[bitsize(32)]
#[derive(Clone, Copy, DebugBits, FromBits, BinaryBits, FromBytes, Immutable, KnownLayout)]
struct Group([bool; 32]);

fn main() {
    let raw_memory = [0u8, 1, 2, 3, 255, 255, 254, 255];
    let mut control = RedistributorControl::read_from_bytes(&raw_memory[0..4]).unwrap();
    let mut group = Group::read_from_bytes(&raw_memory[4..8]).unwrap();

    let redist = Redistributor {
        control: unsafe { VolatilePtr::new(NonNull::from(&mut control)) },
        group: unsafe { VolatilePtr::new_read_only(NonNull::from(&mut group)) },
    };

    // 0_0000_0_1_1_00000010000000010000_0_0_00
    println!("{:032b}", redist.control.read());
    println!("{:?}", redist.control);
    // 11111111111111101111111111111111
    println!("{:032b}", redist.group.read());
    println!("{:?}", redist.group);

    let mut raw_memory: (RedistributorControl, Group) = (0b00000011000000100000000100000000u32.into(), 0b11111111111111101111111111111111u32.into());

    let redist = Redistributor {
        control: unsafe { VolatilePtr::new(NonNull::from(&mut raw_memory.0)) },
        group: unsafe { VolatilePtr::new_read_only(NonNull::from(&mut raw_memory.1)) },
    };

    // 0_0000_0_1_1_00000010000000010000_0_0_00
    println!("{:032b}", redist.control.read());
    println!("{:?}", redist.control);
    // 11111111111111101111111111111111
    println!("{:032b}", redist.group.read());
    println!("{:?}", redist.group);
}
