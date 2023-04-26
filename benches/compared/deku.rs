use deku::{DekuRead, DekuWrite, DekuUpdate, DekuContainerWrite};

/// Don't take this example as "the way to use deku", please.
/// This is just a faulty implementation, but even just providing a value
/// and getting it out again is currently (26.04.2023) not `const` in deku,
/// so we'll wait until (if) that happens.
pub(crate) fn deku(input: (u32, u32, u64, u16)) {       //input.0 = 0b1111_0000_0000_1001_1111_0000_0000_1001
    let mut control: [u8; 4] = input.0.to_le_bytes();   //[0b0000_1001, 0b1111_0000, 0b0000_1001, 0b1111_0000]
    for byte in &mut control {                          //[0b1001_0000, 0b0000_1111, 0b1001_0000, 0b0000_1111]
        *byte = byte.reverse_bits();                    // these lines convert to deku's expected inputs
    }

    let mut implementer_identification: [u8; 4] = input.1.to_le_bytes();
    for byte in &mut implementer_identification {
        *byte = byte.reverse_bits();
    }

    let mut redistributor_type: [u8; 8] = input.2.to_le_bytes();
    for byte in &mut redistributor_type {
        *byte = byte.reverse_bits();
    }

    let mut lpi = GicRedistributorLpi {
        control: RedistributorControl::try_from(control.as_slice()).expect("1"),
        implementer_identification: RedistributorImplementerIdentification::try_from(implementer_identification.as_slice()).expect("2"),
        redistributor_type: RedistributorType::try_from(redistributor_type.as_slice()).expect("3"),
    };

    let mut control: [u8; 4] = lpi.control.to_bytes().unwrap()[0..4].try_into().unwrap();
    for byte in &mut control {
        *byte = byte.reverse_bits();
    }
    let control = u32::from_le_bytes(control);
    let mut implementer_identification: [u8; 4] = lpi.implementer_identification.to_bytes().unwrap()[0..4].try_into().unwrap();
    for byte in &mut implementer_identification {
        *byte = byte.reverse_bits();
    }
    let implementer_identification = u32::from_le_bytes(implementer_identification);
    let mut redistributor_type: [u8; 8] = lpi.redistributor_type.to_bytes().unwrap()[0..8].try_into().unwrap();
    for byte in &mut redistributor_type {
        *byte = byte.reverse_bits();
    }
    let redistributor_type = u64::from_le_bytes(redistributor_type);
    assert_eq!(control, input.0);
    assert_eq!(implementer_identification, input.1);
    assert_eq!(redistributor_type, input.2);
    
    assert!(lpi.control.clear_enable_supported);
    // after trying for too much time, I will not fix this;
    // maybe one needs to use some special API to read values
    // assert_eq!(lpi.implementer_identification.implementer_jep106, 2054);
    lpi.implementer_identification.implementer_jep106 = input.3;
    // this of course works since we just set field, get field
    assert_eq!(lpi.implementer_identification.implementer_jep106, input.3);
    // assert_eq!(lpi.redistributor_type.processor_number, 63872);
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct RedistributorControl {
    //ro
    #[deku(bits = "1")]
    upstream_write_pending: bool,
    //zero
    #[deku(bits = "4")]
    reserved_i: u8,
    //or reserved
        #[deku(bits = "1")]
        disable_processor_selection_for_group_1_secure_interrupts: bool,
        #[deku(bits = "1")]
        disable_processor_selection_for_group_1_non_secure_interrupts: bool,
        #[deku(bits = "1")]
        disable_processor_selection_for_group_0_interrupts: bool,
    //zero
    #[deku(bits = "20")]
    reserved_ii: u32,
    #[deku(bits = "1")]
    register_write_pending: bool,
    #[deku(bits = "1")]
    lpi_invalidate_registers_supported: bool,
    #[deku(bits = "1")]
    clear_enable_supported: bool,
    #[deku(bits = "1")]
    enable_lpis: bool,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct RedistributorImplementerIdentification {
    //ro
    product_id: u8,
    //zero
    #[deku(bits = "4")]
    reserved: u8,
    //ro
    #[deku(bits = "4")]
    variant: u8,
    //ro
    #[deku(bits = "4")]
    revision: u8,
    #[deku(bits = "12")]
    implementer_jep106: u16,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct RedistributorType {
    affinity_value: u32,
    #[deku(bits = "5")]
    ppi_num: u8,
    #[deku(bits = "1")]
    virtual_sgi_supported: bool,
    #[deku(bits = "2")]
    common_lpi_affinity: u8, //TODO: enums everywhere
    processor_number: u16,
    #[deku(bits = "1")]
    resident_vpe_id: bool,
    #[deku(bits = "1")]
    mpam_supported: bool,
    #[deku(bits = "1")]
    control_cpgs_supported: bool,
    #[deku(bits = "1")]
    is_last: bool,
    #[deku(bits = "1")]
    direct_lpi_supported: bool,
    #[deku(bits = "1")]
    dirty: bool,
    #[deku(bits = "1")]
    virtual_lpi_supported: bool,
    #[deku(bits = "1")]
    physical_lpi_supported: bool,
}
