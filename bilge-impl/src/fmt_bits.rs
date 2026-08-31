use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::shared::{self, BitSize, binary_segments, fallback::Fallback, place_struct_fields, unreachable};

pub(crate) fn binary(item: TokenStream) -> manyhow::Result {
    let derive_input = parse(item);
    let (derive_data, _arb_int, name, bitsize, _fallback) = analyze(&derive_input)?;

    match derive_data {
        Data::Struct(data) => Ok(generate_struct_binary_impl(name, &data.fields, bitsize)),
        Data::Enum(_) => Ok(generate_enum_binary_impl(name)),
        _ => unreachable(()),
    }
}

fn generate_struct_binary_impl(struct_name: &Ident, fields: &Fields, declared_bitsize: BitSize) -> TokenStream {
    let layout = place_struct_fields(fields, declared_bitsize as usize).unwrap_or_else(|_| unreachable(()));
    let declared = declared_bitsize as usize;

    // fields (and holes) are printed from most significant to least significant
    let writes = binary_segments(&layout, declared).into_iter().rev().map(|(offset, width)| {
        quote! {
            let field_size = #width;
            if field_size != 0 {
                if started {
                    ::core::write!(f, "_")?;
                }
                started = true;
                let field_mask = mask >> (struct_size - field_size);
                let extracted = field_mask & (self.value >> #offset);
                ::core::write!(f, "{:0width$b}", extracted, width = field_size)?;
            }
        }
    });

    quote! {
        impl ::core::fmt::Binary for #struct_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let struct_size = <#struct_name as Bitsized>::BITS;
                let mask = <#struct_name as Bitsized>::MAX;
                let mut started = false;
                #(#writes)*
                Ok(())
            }
        }
    }
}

fn generate_enum_binary_impl(enum_name: &Ident) -> TokenStream {
    quote! {
        impl ::core::fmt::Binary for #enum_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                use ::bilge::Bitsized as _;
                ::core::write!(
                    f,
                    "{:0width$b}",
                    self.as_int(),
                    width = <#enum_name as Bitsized>::BITS
                )
            }
        }
    }
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> manyhow::Result<(&Data, TokenStream, &Ident, BitSize, Option<Fallback>)> {
    shared::analyze_derive(derive_input, crate::shared::DeriveKind::Other)
}
