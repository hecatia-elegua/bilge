//! Named construction via `#[derive(BuilderBits)]` ([#90](https://github.com/hecatia-elegua/bilge/issues/90)).
#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

use bilge::prelude::*;

/// The normal constructor hides names. If you want to see these names, use `BuilderBits`, which requires each field exactly once.
#[bitsize(32)]
#[derive(Clone, Copy, DebugBits, FromBits, DefaultBits, BuilderBits, PartialEq)]
struct AddInstruction {
    src1: u4,
    reserved: u4,
    // means `ADD` and is optional on the builder.
    #[default(u6::new(8))]
    opcode: u6,
    src2: u4,
    reserved: u2,
    dst: u4,
    reserved: u8,
}

fn main() {
    let src1 = u4::new(1);
    let src2 = u4::new(2);
    let dst = u4::new(3);

    let add = AddInstruction::new(src1, u6::new(8), src2, dst);
    let built = AddInstruction::builder().src1(src1).src2(src2).dst(dst).build();
    assert_eq!(add, built);
    assert_eq!(built.opcode(), u6::new(8));

    // `.src1()` twice, or `.build()` without `dst`, does not compile.
    let other = AddInstruction::builder().src1(src1).opcode(u6::new(9)).src2(src2).dst(dst).build();
    assert_eq!(other.opcode(), u6::new(9));

    // The same `#[default(expr)]` is used by DefaultBits.
    assert_eq!(AddInstruction::default().opcode(), u6::new(8));
}
