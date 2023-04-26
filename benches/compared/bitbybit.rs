use bitbybit::bitfield;
use arbitrary_int::{u4, u12, u20, u5, u2};

pub(crate) fn bitbybit(input: (u32, u32, u64, u16)) {
    let mut lpi = GicRedistributorLpi {
        control: RedistributorControl::new_with_raw_value(input.0),
        implementer_identification: RedistributorImplementerIdentification::new_with_raw_value(input.1),
        redistributor_type: RedistributorType::new_with_raw_value(input.2),
    };
    assert_eq!(lpi.control.raw_value, input.0);
    assert_eq!(lpi.implementer_identification.raw_value, input.1);
    assert_eq!(lpi.redistributor_type.raw_value, input.2);
    
    assert!(lpi.control.clear_enable_supported());
    assert_eq!(lpi.implementer_identification.implementer_jep106(), u12::new(2054));
    lpi.implementer_identification = lpi.implementer_identification.with_implementer_jep106(u12::new(input.3));
    assert_eq!(lpi.implementer_identification.implementer_jep106(), u12::new(input.3));
    assert_eq!(lpi.redistributor_type.processor_number(), 63872);
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[bitfield(u32)]
#[derive(Debug)]
struct RedistributorControl {
    //ro
    #[bit(0, rw)]
    upstream_write_pending: bool,
    //zero
    #[bits(1..=4, rw)]
    reserved_i: u4,
    //or reserved
        #[bit(5, rw)]
        disable_processor_selection_for_group_1_secure_interrupts: bool,
        #[bit(6, rw)]
        disable_processor_selection_for_group_1_non_secure_interrupts: bool,
        #[bit(7, rw)]
        disable_processor_selection_for_group_0_interrupts: bool,
    //zero
    #[bits(8..=27, rw)]
    reserved_ii: u20,
    #[bit(28, rw)]
    register_write_pending: bool,
    #[bit(29, rw)]
    lpi_invalidate_registers_supported: bool,
    #[bit(30, rw)]
    clear_enable_supported: bool,
    #[bit(31, rw)]
    enable_lpis: bool,
}

#[bitfield(u32)]
#[derive(Debug)]
struct RedistributorImplementerIdentification {
    //ro
    #[bits(0..=7, rw)]
    product_id: u8,
    //zero
    #[bits(8..=11, rw)]
    reserved: u4,
    //ro
    #[bits(12..=15, rw)]
    variant: u4,
    //ro
    #[bits(16..=19, rw)]
    revision: u4,
    #[bits(20..=31, rw)]
    implementer_jep106: u12,
}

#[bitfield(u64)]
#[derive(Debug)]
struct RedistributorType {
    #[bits(0..=31, rw)]
    affinity_value: u32,
    #[bits(32..=36, rw)]
    ppi_num: u5,
    #[bit(37, rw)]
    virtual_sgi_supported: bool,
    #[bits(38..=39, rw)]
    common_lpi_affinity: u2, //TODO: enums everywhere
    #[bits(40..=55, rw)]
    processor_number: u16,
    #[bit(56, rw)]
    resident_vpe_id: bool,
    #[bit(57, rw)]
    mpam_supported: bool,
    #[bit(58, rw)]
    control_cpgs_supported: bool,
    #[bit(59, rw)]
    is_last: bool,
    #[bit(60, rw)]
    direct_lpi_supported: bool,
    #[bit(61, rw)]
    dirty: bool,
    #[bit(62, rw)]
    virtual_lpi_supported: bool,
    #[bit(63, rw)]
    physical_lpi_supported: bool,
}
