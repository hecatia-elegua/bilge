#![feature(const_convert, const_trait_impl, const_mut_refs)]
use bilge::prelude::*;

#[bitsize(2)]
#[derive(Debug, Clone, TryFromBits)]
// #[derive(FromBits)]
#[non_exhaustive] //gets handled when FromBits
enum IncompleteEnum {
    A = 0, B = 1, C = 2, //D = 3
}

#[bitsize(2)]
#[derive(FromBits)]
enum CompleteEnum {
    A, B, C, D
}

fn main() {
    let a = IncompleteEnum::try_from(u2::new(0)).unwrap();
    let b: IncompleteEnum = u2::new(0).try_into().unwrap();

    #[allow(unreachable_patterns)]
    match a {
        IncompleteEnum::A => {},
        IncompleteEnum::B => {},
        IncompleteEnum::C => {},
        _ => {},
    }
    #[allow(clippy::redundant_clone)] //actually triggering a clippy bug
    let _: u2 = u2::new(b.clone() as u8);
    let c: u2 = b.clone().into();

    println!("{:?} {:?} {:?}", a, b, c);

    let ada = 1u8;
    #[allow(clippy::match_overlapping_arm)]
    match ada {
        0 => (),
        1 => (),
        2..=u8::MAX => (),
    }

    let a = 2;
    let b = CompleteEnum::from(u2::new(a));
    let c: u2 = b.into();
    println!("{:?}", c);
}
