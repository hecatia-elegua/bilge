use proc_macro2::Literal;
use proc_macro_error::abort;
use syn::{Variant, Expr, ExprLit, Lit};
use super::{BitSize, unreachable};

pub(crate) struct DiscriminantAssigner {
    bitsize: BitSize,
    next_expected_assignment: u128,
}

impl DiscriminantAssigner {
    pub fn new(bitsize: u8) -> DiscriminantAssigner {
        DiscriminantAssigner { bitsize, next_expected_assignment: 0 }
    }
    
    fn max_value(&self) -> u128 {
        (1u128 << self.bitsize) - 1
    }

    fn value_from_discriminant(&self, variant: &Variant) -> Option<u128> {
        let discriminant = variant.discriminant.as_ref()?;
        let discriminant_expr = &discriminant.1;
        let variant_name = &variant.ident;

        let Expr::Lit(ExprLit { lit: Lit::Int(int), .. }) = discriminant_expr else {
            abort!(
                discriminant_expr, 
                "variant `{}` is not a number", variant_name; 
                help = "only literal integers currently supported"
            )
        };
    
        let discriminant_value: u128 = int.base10_parse().unwrap_or_else(unreachable);
        if discriminant_value > self.max_value() {
            abort!(variant, "Value of variant exceeds the given number of bits")
        }

        Some(discriminant_value)
    }

    fn assign(&mut self, variant: &Variant) -> u128 {
        let value = self.value_from_discriminant(variant).unwrap_or(self.next_expected_assignment);
        self.next_expected_assignment = value + 1;
        value
    }

    /// syn adds a suffix when printing Rust integers. we use an unsuffixed `Literal` for better-looking codegen
    pub fn assign_unsuffixed(&mut self, variant: &Variant) -> Literal {
        let next = self.assign(variant);
        Literal::u128_unsuffixed(next)
    }
}