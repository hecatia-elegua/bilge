use bilge::prelude::*;

#[bitsize(16)]
#[derive(FromBits, DebugBits)]
struct EthercatHeader1 {
	len: u11,
	some: u1,
	ty: u4,
}

#[bitsize(16)]
#[derive(FromBits, DebugBits)]
struct EthercatHeader2 {
	len: u11,
	reserved: u1,
	ty: u4,
}

#[test]
fn should_be_same_structure_issue_30() {
    let ty = u4::new(0x1);
    let eh1 = EthercatHeader1::new(u11::new(0xe), u1::new(0), ty);
    let eh2 = EthercatHeader2::new(u11::new(0xe), ty);
    assert_eq!(eh1.ty(), ty);
    assert_eq!(eh2.ty(), ty);
    assert_eq!(eh1.value, eh2.value);
}
