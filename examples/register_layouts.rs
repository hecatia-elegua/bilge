//! Sketches for bit-range placement ([#28](https://github.com/hecatia-elegua/bilge/issues/28))
//! and overlapping layouts ([#91](https://github.com/hecatia-elegua/bilge/issues/91)).
//!
#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]

use bilge::prelude::*;

/// Datasheet-style holes. `#[at(n)]` is backing-integer bit n (0 = LSB).
/// Sequential fields without `#[at]` still fill from bit 0 upward.
/// Overlap is a compile error; skipped bits are implicit padding.
#[bitsize(16)]
#[derive(Clone, Copy, DebugBits, FromBits)]
struct Status {
    #[at(3)]
    pub ready: bool, // bit 3; bits 0..=2 implicit padding
    #[at(8..=11)]
    pub nack: u4, // bits 8..=11; bits 4..=7 and 12..=15 implicit padding
}

/// RISC-V base instruction formats share a 32-bit word. Opcode (bits 0..=6) is the tag
/// *in the same integer*. Payload is the other 25 bits.
#[bitsize(25)]
#[derive(Clone, Copy, DebugBits, FromBits, PartialEq)]
struct RTypeRest {
    pub rd: u5,
    pub funct3: u3,
    pub rs1: u5,
    pub rs2: u5,
    pub funct7: u7,
}

#[bitsize(25)]
#[derive(Clone, Copy, DebugBits, FromBits, PartialEq)]
struct ITypeRest {
    pub rd: u5,
    pub funct3: u3,
    pub rs1: u5,
    pub imm: u12,
}

#[bitsize(32)]
#[discriminant_at(0..=6)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromBits)]
enum Instr {
    R(RTypeRest) = 0b0110011,
    I(ITypeRest) = 0b0010011,
}

// TODO: `reg.value()` instead of `u8::from(reg)` if we do decide on adding that method for all enums
/// The tag is a different MMIO register, not bits inside the data byte.
/// VGA CRTC: write an index to 0x3D4, then 0x3D5 is that register's byte.
///
/// `#[discriminant(CrtcIndex)]`: tag is a separate value of that type.
/// `#[bitsize]` is the payload only; variant `= n` is the tag.
/// Contrast `#[discriminant_at(0..=6)]`, where the tag is bits *in* the integer.
///
/// This is basically `TryFrom<(tag, bits)>` because unused index values exist;
/// the write direction is always `From`.
/// ```ignore
/// #[bitsize(8)]
/// #[discriminant(CrtcIndex)]
/// enum CrtcReg {
///     Horiz(HorizontalDisplayEnd) = 0x01,
///     MaxScan(MaxScanLine) = 0x09,
/// }
///
/// let reg = CrtcReg::try_from((index, data))?;        // read: 0x3D4 then 0x3D5
/// let (index, data) = <(CrtcIndex, u8)>::from(reg);   // write
/// mmio.index.write(index);
/// mmio.data.write(data);
/// ```
#[bitsize(8)]
#[derive(Clone, Copy, DebugBits, FromBits)]
struct CrtcIndex {
    pub index: u8,
}

#[bitsize(8)]
#[derive(Clone, Copy, DebugBits, FromBits, PartialEq)]
struct HorizontalDisplayEnd {
    pub width_minus_1: u8,
}

#[bitsize(8)]
#[derive(Clone, Copy, DebugBits, FromBits, PartialEq)]
struct MaxScanLine {
    pub max_scan: u5,
    padding: u3,
}

/// Today's stand-in for the generated enum: tag is *next to* the payload.
#[derive(Clone, Copy, Debug, PartialEq)]
enum CrtcReg {
    Horiz(HorizontalDisplayEnd),
    MaxScan(MaxScanLine),
}

fn crtc_from_tag(index: CrtcIndex, data: u8) -> Result<CrtcReg, ()> {
    match index.index() {
        0x01 => Ok(CrtcReg::Horiz(HorizontalDisplayEnd::from(data))),
        0x09 => Ok(CrtcReg::MaxScan(MaxScanLine::from(data))),
        _ => Err(()),
    }
}

fn crtc_to_tag(reg: CrtcReg) -> (CrtcIndex, u8) {
    match reg {
        CrtcReg::Horiz(h) => (CrtcIndex::from(0x01), u8::from(h)),
        CrtcReg::MaxScan(m) => (CrtcIndex::from(0x09), u8::from(m)),
    }
}

fn main() {
    let st = Status::from(u16::new(1 << 3 | (0xA << 8)));
    assert!(st.ready());
    assert_eq!(st.nack(), u4::new(0xA));

    // addi x1, x0, 5 - opcode OP-IMM, rd=1, funct3=0, rs1=0, imm=5
    let addi = 0x0050_0093;
    let Instr::I(i) = Instr::try_from(u32::new(addi)).unwrap() else {
        panic!("expected I-type");
    };
    assert_eq!(i.rd().value(), 1);
    assert_eq!(i.imm().value(), 5);
    assert_eq!(u32::from(Instr::I(i)).value(), addi);

    let reg = crtc_from_tag(CrtcIndex::from(0x01), 79).unwrap();
    assert_eq!(reg, CrtcReg::Horiz(HorizontalDisplayEnd::from(79)));
    let (index, data) = crtc_to_tag(reg);
    assert_eq!(index.index(), 0x01);
    assert_eq!(data, 79);

    let reg = CrtcReg::MaxScan(MaxScanLine::from(0b000_01111));
    let (index, data) = crtc_to_tag(reg);
    assert_eq!(crtc_from_tag(index, data).unwrap(), reg);
}
