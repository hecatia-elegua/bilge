// we need to put this on the whole module since bitfield doesn't pass it through
// (into_bytes, new is never used)
#![allow(dead_code)]
use modular_bitfield::{bitfield, specifiers::{B20, B4, B12, B5, B2}, Specifier};

#[inline(never)]
pub(crate) fn modular(input: (u32, u32, u64, u16)) {
    // convert to modular_bitfield's expected inputs
    let control: [u8; 4] = input.0.to_le_bytes();
    let implementer_identification: [u8; 4] = input.1.to_le_bytes();
    let redistributor_type: [u8; 8] = input.2.to_le_bytes();

    let mut lpi = GicRedistributorLpi {
        control: RedistributorControl::from_bytes(control),
        implementer_identification: RedistributorImplementerIdentification::from_bytes(implementer_identification),
        redistributor_type: RedistributorType::from_bytes(redistributor_type),
    };
    assert_eq!(u32::from_le_bytes(lpi.control.bytes), input.0);
    assert_eq!(u32::from_le_bytes(lpi.implementer_identification.bytes), input.1);
    assert_eq!(u64::from_le_bytes(lpi.redistributor_type.bytes), input.2);
    
    assert!(lpi.control.clear_enable_supported());
    assert_eq!(lpi.implementer_identification.implementer_jep106(), 2054); //B12?
    lpi.implementer_identification.set_implementer_jep106(B12::from_bytes(input.3).unwrap()); //`from_bytes` does not always get bytes..
    assert_eq!(lpi.implementer_identification.implementer_jep106(), input.3); //B12?
    assert_eq!(lpi.redistributor_type.processor_number(), 63872);
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[bitfield(bits = 32)]
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
struct RedistributorType {
    affinity_value: u32,
    ppi_num: B5,
    virtual_sgi_supported: bool,
    common_lpi_affinity: B2, //TODO: enums everywhere
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
