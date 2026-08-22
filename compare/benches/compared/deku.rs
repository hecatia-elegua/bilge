use deku::prelude::*;

use super::Input;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u8",
    bits = 2,
    endian = "endian",
    bit_order = "order",
    ctx = "endian: deku::ctx::Endian, order: deku::ctx::Order"
)]
#[repr(u8)]
pub enum CommonLpiAffinity {
    All = 0,
    Core = 1,
    Cluster = 2,
    Reserved = 3,
}

fn parse<'a, T: DekuContainerRead<'a>>(bytes: &'a [u8]) -> T {
    let (_, value) = T::from_bytes((bytes, 0)).expect("valid register encoding");
    value
}

fn to_u32(bytes: Vec<u8>) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("4 bytes"))
}

fn to_u64(bytes: Vec<u8>) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("8 bytes"))
}

fn construct(input: Input) -> GicRedistributorLpi {
    GicRedistributorLpi {
        control: parse(&input.0.to_le_bytes()),
        implementer_identification: parse(&input.1.to_le_bytes()),
        redistributor_type: parse(&input.2.to_le_bytes()),
    }
}

pub fn from_raw(input: Input) -> GicRedistributorLpi {
    construct(input)
}

pub fn getters(lpi: &GicRedistributorLpi) -> (bool, u16, u16, CommonLpiAffinity) {
    (
        lpi.control.clear_enable_supported,
        lpi.implementer_identification.implementer_jep106,
        lpi.redistributor_type.processor_number,
        lpi.redistributor_type.common_lpi_affinity,
    )
}

pub fn set_jep106(lpi: &mut GicRedistributorLpi, value: u16) {
    lpi.implementer_identification.implementer_jep106 = value;
}

#[inline(never)]
pub fn combined(input: Input) -> (u32, u32, u64, bool, u16, u16, CommonLpiAffinity) {
    let mut lpi = construct(input);
    let clear = lpi.control.clear_enable_supported;
    let affinity = lpi.redistributor_type.common_lpi_affinity;
    let processor_number = lpi.redistributor_type.processor_number;
    lpi.implementer_identification.implementer_jep106 = input.3;
    let jep106 = lpi.implementer_identification.implementer_jep106;
    (
        to_u32(lpi.control.to_bytes().unwrap()),
        to_u32(lpi.implementer_identification.to_bytes().unwrap()),
        to_u64(lpi.redistributor_type.to_bytes().unwrap()),
        clear,
        jep106,
        processor_number,
        affinity,
    )
}

pub fn check(input: Input) {
    let lpi = from_raw(input);
    assert_eq!(to_u32(lpi.control.to_bytes().unwrap()), input.0);
    assert_eq!(to_u32(lpi.implementer_identification.to_bytes().unwrap()), input.1);
    assert_eq!(to_u64(lpi.redistributor_type.to_bytes().unwrap()), input.2);
    assert!(lpi.control.clear_enable_supported);
    assert_eq!(lpi.implementer_identification.implementer_jep106, 2054);
    assert_eq!(lpi.redistributor_type.processor_number, 63872);
    let _ = lpi.redistributor_type.common_lpi_affinity;

    let mut lpi = lpi;
    set_jep106(&mut lpi, input.3);
    assert_eq!(lpi.implementer_identification.implementer_jep106, input.3);
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct RedistributorControl {
    //ro
    #[deku(bits = 1)]
    upstream_write_pending: bool,
    //zero
    #[deku(bits = 4)]
    reserved_i: u8,
    //or reserved
    #[deku(bits = 1)]
    disable_processor_selection_for_group_1_secure_interrupts: bool,
    #[deku(bits = 1)]
    disable_processor_selection_for_group_1_non_secure_interrupts: bool,
    #[deku(bits = 1)]
    disable_processor_selection_for_group_0_interrupts: bool,
    //zero
    #[deku(bits = 20)]
    reserved_ii: u32,
    #[deku(bits = 1)]
    register_write_pending: bool,
    #[deku(bits = 1)]
    lpi_invalidate_registers_supported: bool,
    #[deku(bits = 1)]
    clear_enable_supported: bool,
    #[deku(bits = 1)]
    enable_lpis: bool,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct RedistributorImplementerIdentification {
    //ro
    product_id: u8,
    //zero
    #[deku(bits = 4)]
    reserved: u8,
    //ro
    #[deku(bits = 4)]
    variant: u8,
    //ro
    #[deku(bits = 4)]
    revision: u8,
    #[deku(bits = 12)]
    implementer_jep106: u16,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct RedistributorType {
    affinity_value: u32,
    #[deku(bits = 5)]
    ppi_num: u8,
    #[deku(bits = 1)]
    virtual_sgi_supported: bool,
    common_lpi_affinity: CommonLpiAffinity,
    processor_number: u16,
    #[deku(bits = 1)]
    resident_vpe_id: bool,
    #[deku(bits = 1)]
    mpam_supported: bool,
    #[deku(bits = 1)]
    control_cpgs_supported: bool,
    #[deku(bits = 1)]
    is_last: bool,
    #[deku(bits = 1)]
    direct_lpi_supported: bool,
    #[deku(bits = 1)]
    dirty: bool,
    #[deku(bits = 1)]
    virtual_lpi_supported: bool,
    #[deku(bits = 1)]
    physical_lpi_supported: bool,
}
