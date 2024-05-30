use bilge::prelude::*;

mod hidden {
    use super::*;
    #[bitsize(96)]
    #[derive(FromBits)]
    pub struct Diary(u48, pub u48);

    #[bitsize(8)]
    #[derive(FromBits)]
    pub struct ArrayAt(pub [u2; 2], [u2; 2]);

    #[bitsize(128)]
    #[derive(FromBits)]
    pub struct Reserver {
        pub reserved: u8,
        pub(crate) padding: u8,
        reserved: u16,
        // the best use for padding
        pub padding: Diary,
    }
}
use hidden::*;

fn main() {
    let ascii = u48::new(0b01001001_00100000_01101100_01101111_01110110_01100101);
    let rusti = u48::new(0b00100000_01110010_01110101_01110011_01110100_00101110);
    let mut diary = Diary::new(ascii, rusti);
    diary.val_0();
    diary.val_1();
    diary.set_val_0(rusti);
    diary.set_val_1(ascii);

    let mut a = ArrayAt::from(42);
    a.val_0_at(1);
    a.set_val_0_at(1, u2::new(0));
    a.val_1_at(1);
    a.set_val_1_at(1, u2::new(0));

    // NOTE: as noted in another test case, `reserved` fields should mean "never used" and
    // might soon only be available to `DebugBits`, unless you need it. Then please open a github issue.
    // Right now though they have getters and do respect visibility:
    let mut r = Reserver::from(42);
    r.reserved_i();
    r.padding_i();
    r.reserved_ii();
    r.padding_ii();
}
