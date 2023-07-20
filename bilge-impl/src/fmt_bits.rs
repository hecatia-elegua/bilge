use crate::shared::{self, unreachable, BitSize, discriminant_assigner::DiscriminantAssigner, fallback::Fallback};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Variant, punctuated::Iter};

pub(crate) fn binary(item: TokenStream) -> TokenStream {
    let derive_input = parse(item);
    let (derive_data, arb_int, name, bitsize, fallback) = analyze(&derive_input);

    match derive_data {
        Data::Struct(data) => generate_struct_binary_impl(name, &data.fields),
        Data::Enum(data) => generate_enum_binary_impl(name, data.variants.iter(), arb_int, bitsize, fallback),
        _ => unreachable(()),
    }
}

fn generate_struct_binary_impl(struct_name: &Ident, fields: &Fields) -> TokenStream {
    let write_underscore = quote! { write!(f, "_")?; };

    // fields are printed from most significant to least significant, separated by an underscore
    let writes = fields
        .iter()
        .rev()
        .map(|field| {
            let field_size = shared::generate_type_bitsize(&field.ty);

            // `extracted` is `field_size` bits of `value`, starting from index `first_bit_pos` (counting from LSB)
            quote! {
                let field_size = #field_size;
                let field_mask = mask >> (struct_size - field_size);
                let first_bit_pos = last_bit_pos - field_size;
                last_bit_pos -= field_size;
                let extracted = field_mask & (self.value >> first_bit_pos);
                write!(f, "{:0width$b}", extracted, width = field_size)?;
            }
        })
        .reduce(|acc, next| quote!(#acc #write_underscore #next));

    quote! {
        impl ::core::fmt::Binary for #struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let struct_size = <#struct_name as Bitsized>::BITS;
                let mut last_bit_pos = struct_size;
                let mask = <#struct_name as Bitsized>::MAX;
                #writes
                Ok(())
            }
        }
    }
}

fn generate_enum_binary_impl(enum_name: &Ident, variants: Iter<Variant>, arb_int: TokenStream, bitsize: BitSize, fallback: Option<Fallback>) -> TokenStream {
    let to_int_match_arms = generate_to_int_match_arms(variants, enum_name, bitsize, arb_int, fallback);

    let body = if to_int_match_arms.is_empty() {
        quote! { Ok(()) }
    } else {
        quote! {
            let value = match self {
                #( #to_int_match_arms )*
            };
            write!(f, "{:0width$b}", value, width = <#enum_name as Bitsized>::BITS)
        }
    };

    quote! {
        impl ::core::fmt::Binary for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #body
            }
        }
    }
}

/// generates the arms for an (infallible) conversion from an enum to the enum's underlying arbitrary_int
fn generate_to_int_match_arms(variants: Iter<Variant>, enum_name: &Ident, bitsize: BitSize, arb_int: TokenStream, fallback: Option<Fallback>) -> Vec<TokenStream> {
    let is_value_fallback = |variant_name| if let Some(Fallback::WithValue(name)) = &fallback {
        variant_name == name
    } else {
        false
    };

    let mut assigner = DiscriminantAssigner::new(bitsize);

    variants
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_value = assigner.assign_unsuffixed(variant);

            if is_value_fallback(variant_name) {
                quote! { #enum_name::#variant_name(number) => *number, }
            } else {
                shared::to_int_match_arm(enum_name, variant_name, &arb_int, variant_value)
            }
        })
        .collect()
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}

fn analyze(derive_input: &DeriveInput) -> (&Data, TokenStream, &Ident, BitSize, Option<Fallback>) {
    shared::analyze_derive(derive_input, false)
}
