#![allow(dead_code)]
use modular_bitfield::prelude::*;

use super::Input;

#[derive(Specifier, Debug, Clone, Copy, PartialEq, Eq)]
#[bits = 2]
pub enum CommonLpiAffinity {
    All = 0,
    Core = 1,
    Cluster = 2,
    Reserved = 3,
}

fn construct(input: Input) -> GicRedistributorLpi {
    GicRedistributorLpi {
        control: RedistributorControl::from_bytes(input.0.to_le_bytes()),
        implementer_identification: RedistributorImplementerIdentification::from_bytes(input.1.to_le_bytes()),
        redistributor_type: RedistributorType::from_bytes(input.2.to_le_bytes()),
    }
}

pub fn from_raw(input: Input) -> GicRedistributorLpi {
    construct(input)
}

pub fn getters(lpi: &GicRedistributorLpi) -> (bool, u16, u16, CommonLpiAffinity) {
    (
        lpi.control.clear_enable_supported(),
        lpi.implementer_identification.implementer_jep106(),
        lpi.redistributor_type.processor_number(),
        lpi.redistributor_type.common_lpi_affinity(),
    )
}

pub fn set_jep106(lpi: &mut GicRedistributorLpi, value: u16) {
    lpi.implementer_identification.set_implementer_jep106(value);
}

#[inline(never)]
pub fn combined(input: Input) -> (u32, u32, u64, bool, u16, u16, CommonLpiAffinity) {
    let mut lpi = construct(input);
    let clear = lpi.control.clear_enable_supported();
    let affinity = lpi.redistributor_type.common_lpi_affinity();
    let processor_number = lpi.redistributor_type.processor_number();
    lpi.implementer_identification.set_implementer_jep106(input.3);
    let jep106 = lpi.implementer_identification.implementer_jep106();
    (
        u32::from_le_bytes(lpi.control.into_bytes()),
        u32::from_le_bytes(lpi.implementer_identification.into_bytes()),
        u64::from_le_bytes(lpi.redistributor_type.into_bytes()),
        clear,
        jep106,
        processor_number,
        affinity,
    )
}

pub fn check(input: Input) {
    let lpi = from_raw(input);
    assert_eq!(u32::from_le_bytes(lpi.control.into_bytes()), input.0);
    assert_eq!(u32::from_le_bytes(lpi.implementer_identification.into_bytes()), input.1);
    assert_eq!(u64::from_le_bytes(lpi.redistributor_type.into_bytes()), input.2);
    assert!(lpi.control.clear_enable_supported());
    assert_eq!(lpi.implementer_identification.implementer_jep106(), 2054);
    assert_eq!(lpi.redistributor_type.processor_number(), 63872);
    let _ = lpi.redistributor_type.common_lpi_affinity();

    let mut lpi = from_raw(input);
    set_jep106(&mut lpi, input.3);
    assert_eq!(lpi.implementer_identification.implementer_jep106(), input.3);
}

#[derive(Debug, Clone, Copy)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy)]
#[rustfmt::skip]
struct RedistributorControl {
    //ro
    upstream_write_pending: bool,
    //zero
    reserved_i: B4,
    //or reserved
        disable_processor_selection_for_group_1_secure_interrupts: bool,
        disable_processor_selection_for_group_1_non_secure_interrupts: bool,
        disable_processor_selection_for_group_0_interrupts: bool,
    //zero
    reserved_ii: B20,
    register_write_pending: bool,
    lpi_invalidate_registers_supported: bool,
    clear_enable_supported: bool,
    enable_lpis: bool,
}

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy)]
struct RedistributorImplementerIdentification {
    //ro
    product_id: u8,
    //zero
    reserved: B4,
    //ro
    variant: B4,
    //ro
    revision: B4,
    implementer_jep106: B12,
}

#[bitfield(bits = 64)]
#[derive(Debug, Clone, Copy)]
struct RedistributorType {
    affinity_value: u32,
    ppi_num: B5,
    virtual_sgi_supported: bool,
    common_lpi_affinity: CommonLpiAffinity,
    processor_number: u16,
    resident_vpe_id: bool,
    mpam_supported: bool,
    control_cpgs_supported: bool,
    is_last: bool,
    direct_lpi_supported: bool,
    dirty: bool,
    virtual_lpi_supported: bool,
    physical_lpi_supported: bool,
}
