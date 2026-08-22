use bilge::prelude::*;

mod inner {
    use super::*;

    #[bitsize(8)]
    #[derive(FromBits)]
    pub struct DefaultPrivateNew {
        pub a: u4,
        pub b: u4,
    }

    #[bitsize(8, new = pub(self))]
    #[derive(FromBits)]
    pub struct ExplicitSelfNew {
        pub a: u4,
        pub b: u4,
    }
}

fn main() {
    let _ = inner::DefaultPrivateNew::new(u4::new(0), u4::new(0));
    let _ = inner::ExplicitSelfNew::new(u4::new(0), u4::new(0));
}
