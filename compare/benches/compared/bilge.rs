use bilge::prelude::*;

use super::Input;

#[bitsize(2)]
#[derive(FromBits, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonLpiAffinity {
    All,
    Core,
    Cluster,
    Reserved,
}

fn construct(input: Input) -> GicRedistributorLpi {
    GicRedistributorLpi {
        control: RedistributorControl::from(input.0),
        implementer_identification: RedistributorImplementerIdentification::from(input.1),
        redistributor_type: RedistributorType::from(input.2),
    }
}

pub fn from_raw(input: Input) -> GicRedistributorLpi {
    construct(input)
}

pub fn getters(lpi: &GicRedistributorLpi) -> (bool, u12, u16, CommonLpiAffinity) {
    (
        lpi.control.clear_enable_supported(),
        lpi.implementer_identification.implementer_jep106(),
        lpi.redistributor_type.processor_number(),
        lpi.redistributor_type.common_lpi_affinity(),
    )
}

pub fn set_jep106(lpi: &mut GicRedistributorLpi, value: u16) {
    lpi.implementer_identification.set_implementer_jep106(u12::new(value));
}

#[inline(never)]
pub fn combined(input: Input) -> (u32, u32, u64, bool, u12, u16, CommonLpiAffinity) {
    let mut lpi = construct(input);
    let clear = lpi.control.clear_enable_supported();
    let affinity = lpi.redistributor_type.common_lpi_affinity();
    let processor_number = lpi.redistributor_type.processor_number();
    lpi.implementer_identification.set_implementer_jep106(u12::new(input.3));
    let jep106 = lpi.implementer_identification.implementer_jep106();
    (
        lpi.control.value,
        lpi.implementer_identification.value,
        lpi.redistributor_type.value,
        clear,
        jep106,
        processor_number,
        affinity,
    )
}

pub fn check(input: Input) {
    let lpi = from_raw(input);
    assert_eq!(lpi.control.value, input.0);
    assert_eq!(lpi.implementer_identification.value, input.1);
    assert_eq!(lpi.redistributor_type.value, input.2);
    assert!(lpi.control.clear_enable_supported());
    assert_eq!(lpi.implementer_identification.implementer_jep106(), u12::new(2054));
    assert_eq!(lpi.redistributor_type.processor_number(), 63872);
    let _ = lpi.redistributor_type.common_lpi_affinity();

    let mut lpi = lpi;
    set_jep106(&mut lpi, input.3);
    assert_eq!(lpi.implementer_identification.implementer_jep106(), u12::new(input.3));
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[bitsize(32)]
#[derive(DebugBits, FromBits)]
#[rustfmt::skip]
struct RedistributorControl {
    //ro
    upstream_write_pending: bool,
    //zero
    reserved: u4,
    //or reserved
        disable_processor_selection_for_group_1_secure_interrupts: bool,
        disable_processor_selection_for_group_1_non_secure_interrupts: bool,
        disable_processor_selection_for_group_0_interrupts: bool,
    //zero
    reserved: u20,
    register_write_pending: bool,
    lpi_invalidate_registers_supported: bool,
    clear_enable_supported: bool,
    enable_lpis: bool,
}

#[bitsize(32)]
#[derive(DebugBits, FromBits)]
struct RedistributorImplementerIdentification {
    //ro
    product_id: u8,
    //zero
    reserved: u4,
    //ro
    variant: u4,
    //ro
    revision: u4,
    implementer_jep106: u12,
}

#[bitsize(64)]
#[derive(DebugBits, FromBits)]
struct RedistributorType {
    affinity_value: u32,
    ppi_num: u5,
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
