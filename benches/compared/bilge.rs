use bilge::{Bitsized, Number, FromBits, bitsize, u4, u12, u20, u5, u2, DebugBits};

#[inline(never)]
pub fn bilge(input: (u32, u32, u64, u16)) {
    let mut lpi = GicRedistributorLpi {
        control: RedistributorControl::from(input.0),
        implementer_identification: RedistributorImplementerIdentification::from(input.1),
        redistributor_type: RedistributorType::from(input.2),
    };
    assert_eq!(lpi.control.value, input.0);
    assert_eq!(lpi.implementer_identification.value, input.1);
    assert_eq!(lpi.redistributor_type.value, input.2);
    
    assert!(lpi.control.clear_enable_supported());
    assert_eq!(lpi.implementer_identification.implementer_jep106(), u12::new(2054));
    lpi.implementer_identification.set_implementer_jep106(u12::new(input.3));
    assert_eq!(lpi.implementer_identification.implementer_jep106(), u12::new(input.3));
    assert_eq!(lpi.redistributor_type.processor_number(), 63872);
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[bitsize(32)]
#[derive(DebugBits, FromBits)]
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
    common_lpi_affinity: u2, //TODO: enums everywhere
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
