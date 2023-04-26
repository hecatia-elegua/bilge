#![allow(dead_code)]

//TODO: version with arbints or range checking? (to just get its benefits)
pub(crate) fn handmade(input: (u32, u32, u64, u16)) {
    let mut lpi = GicRedistributorLpi {
        control: RedistributorControl(input.0),
        implementer_identification: RedistributorImplementerIdentification(input.1),
        redistributor_type: RedistributorType(input.2),
    };
    assert_eq!(lpi.control.0, input.0);
    assert_eq!(lpi.implementer_identification.0, input.1);
    assert_eq!(lpi.redistributor_type.0, input.2);
    
    assert!(lpi.control.clear_enable_supported());
    assert_eq!(lpi.implementer_identification.implementer_jep106(), 2054);
    lpi.implementer_identification.set_implementer_jep106(input.3);
    assert_eq!(lpi.implementer_identification.implementer_jep106(), input.3);
    assert_eq!(lpi.redistributor_type.processor_number(), 63872);
}

#[derive(Debug)]
pub struct GicRedistributorLpi {
    control: RedistributorControl,
    implementer_identification: RedistributorImplementerIdentification,
    redistributor_type: RedistributorType,
}

#[derive(Debug)]
struct RedistributorControl(u32);
impl RedistributorControl {
    //ro
    const fn upstream_write_pending(&self) -> bool {
        self.0 & 1 != 0
    }
    //zero
    //u4
    const fn reserved_i(&self) -> u8 {
        (self.0 >> 1) as u8 & 0b1111
    }
    //or reserved
        const fn disable_processor_selection_for_group_1_secure_interrupts(&self) -> bool {
            (self.0 >> 5) & 1 != 0

        }
        const fn disable_processor_selection_for_group_1_non_secure_interrupts(&self) -> bool {
            (self.0 >> 6) & 1 != 0
        }
        const fn disable_processor_selection_for_group_0_interrupts(&self) -> bool {
            (self.0 >> 7) & 1 != 0
        }
    //zero
    //u20
    const fn reserved_ii(&self) -> u32 {
        (self.0 >> 8) & 0b1111_1111_1111_1111_1111
    }
    const fn register_write_pending(&self) -> bool {
        (self.0 >> 27) & 1 != 0
    }
    const fn lpi_invalidate_registers_supported(&self) -> bool {
        (self.0 >> 28) & 1 != 0
    }
    const fn clear_enable_supported(&self) -> bool {
        (self.0 >> 29) & 1 != 0
    }
    const fn enable_lpis(&self) -> bool {
        (self.0 >> 30) & 1 != 0
    }
}

#[derive(Debug)]
struct RedistributorImplementerIdentification(u32);
impl RedistributorImplementerIdentification {
    //ro
    const fn product_id(&self) -> u8 {
        self.0 as u8
    }
    //zero
    //u4
    const fn reserved_i(&self) -> u8 {
        (self.0 >> 8) as u8 & 0b1111
    }
    //ro
    //u4
    const fn variant(&self) -> u8 {
        (self.0 >> 12) as u8 & 0b1111
    }
    //ro
    //u4
    const fn revision(&self) -> u8 {
        (self.0 >> 16) as u8 & 0b1111
    }
    //u12
    const fn implementer_jep106(&self) -> u16 {
        (self.0 >> 20) as u16 & 0b1111_1111_1111
    }
    //u12
    const fn set_implementer_jep106(&mut self, value: u16) {
        let this_value = ((value & 0b1111_1111_1111) as u32) << 20;
        let others_mask = !(0b1111_1111_1111 << 20);
        let other_values = self.0 & others_mask;
        self.0 = other_values | this_value;
    }
}

#[derive(Debug)]
struct RedistributorType(u64);
impl RedistributorType {
    const fn affinity_value(&self) -> u32 {
        self.0 as u32
    }
    //u5
    const fn ppi_num(&self) -> u8 {
        (self.0 >> 32) as u8 & 0b1_1111
    }
    const fn virtual_sgi_supported(&self) -> bool {
        (self.0 >> 37) & 1 != 0
    }
    //u2
    const fn common_lpi_affinity(&self) -> u8 {
        (self.0 >> 38) as u8 & 0b11
    }
    const fn processor_number(&self) -> u16 {
        (self.0 >> 40) as u16
    }
    const fn resident_vpe_id(&self) -> bool {
        (self.0 >> 56) & 1 != 0
    }
    const fn mpam_supported(&self) -> bool {
        (self.0 >> 57) & 1 != 0
    }
    const fn control_cpgs_supported(&self) -> bool {
        (self.0 >> 58) & 1 != 0
    }
    const fn is_last(&self) -> bool {
        (self.0 >> 59) & 1 != 0
    }
    const fn direct_lpi_supported(&self) -> bool {
        (self.0 >> 60) & 1 != 0
    }
    const fn dirty(&self) -> bool {
        (self.0 >> 61) & 1 != 0
    }
    const fn virtual_lpi_supported(&self) -> bool {
        (self.0 >> 62) & 1 != 0
    }
    const fn physial_lpi_supported(&self) -> bool {
        (self.0 >> 63) & 1 != 0
    }
}
