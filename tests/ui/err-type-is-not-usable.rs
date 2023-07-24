#![feature(const_trait_impl)] use bilge::prelude::*;

#[bitsize(2)]
#[derive(TryFromBits)]
enum Dna {
    C,
    G,
    A,
}

fn main() {
    let res = Dna::try_from(u2::new(3));
    match res {
        Ok(Dna::C) => println!("cytosine"),
        Ok(Dna::G) => println!("guanine"),
        Ok(Dna::A) => println!("adenine"),
        Err(bilge::BitsError) => println!("thymine"),
    }
}
