#![cfg_attr(feature = "nightly", feature(const_convert, const_trait_impl, const_mut_refs))]
#![allow(clippy::unusual_byte_groupings)]
use bilge::prelude::*;

#[bitsize(24)]
#[derive(DebugBits, FromBits)]
pub struct PS2MousePacket {
    pub button_left: bool,
    pub button_right: bool,
    pub button_middle: bool,
    pub always_one: u1,
    x_9th_bit: bool,
    y_9th_bit: bool,
    x_overflow: bool,
    y_overflow: bool,
    x_1st_to_8th_bit: u8,
    y_1st_to_8th_bit: u8,
}

fn main() {
    let value_from_port = 0b11100111_00001111_00111001;
    let mouse_packet = PS2MousePacket::from(u24::new(value_from_port));
    // PS2MousePacket {
    //     button_left: true, button_right: false, button_middle: false, always_one: 1,
    //     x_9th_bit: true, y_9th_bit: true, x_overflow: false, y_overflow: false,
    //     x_1st_to_8th_bit: 15, y_1st_to_8th_bit: 231
    // }
    println!("{mouse_packet:?}");
    assert_eq!(mouse_packet.x_1st_to_8th_bit(), 0b00001111);
    assert_eq!(mouse_packet.y_1st_to_8th_bit(), 0b11100111);
    assert_eq!(mouse_packet.always_one().value(), 1);
    assert!(mouse_packet.button_left());
    assert!(mouse_packet.x_9th_bit());
}
