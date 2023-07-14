use super::{BitSize, is_fallback_attribute, unreachable, bitsize_from_type_token};
use itertools::Itertools;
use proc_macro2::Ident;
use proc_macro_error::{abort, abort_call_site};
use syn::{Type, Variant, Data};

pub enum Fallback {
    Unit(Ident),
    WithValue(Ident),
}

impl Fallback {
    fn from_variant(variant: &Variant, enum_bitsize: BitSize, is_last_variant: bool) -> Fallback {
        use syn::Fields::*;

        let ident = variant.ident.to_owned();

        match &variant.fields {
            Named(_) => {
                abort!(variant, "`#[fallback]` does not support variants with named fields"; help = "use a tuple variant or remove this `#[fallback]`")
            }
            Unnamed(fields) => {
                let variant_fields = fields.unnamed.iter();
                let Ok(fallback_value) = variant_fields.exactly_one() else {
                    abort!(variant, "fallback variant must have exactly one field"; help = "use only one field or change to a unit variant")
                };
                
                if !is_last_variant {
                    abort!(variant, "value fallback is not the last variant"; help = "a fallback variant with value must be the last variant of the enum")
                }

                let Type::Path(type_path) = &fallback_value.ty else {
                    abort!(variant.fields, "`#[fallback]` only supports arbitrary_int or bool types")
                };

                // here we validate that the fallback variant field type matches the bitsize
                match bitsize_from_type_token(&type_path.path) {
                    Some(bitsize) if bitsize == enum_bitsize => Fallback::WithValue(ident),
                    Some(bitsize) => abort!(
                        variant.fields,
                        "bitsize of fallback field ({}) does not match bitsize of enum ({})",
                        bitsize,
                        enum_bitsize
                    ),
                    None => abort!(variant.fields, "`#[fallback]` only supports arbitrary_int or bool types"),
                }
            }
            Unit => Fallback::Unit(ident),
        }
    }

    pub fn is_fallback_variant(&self, variant_ident: &Ident) -> bool {
        matches!(self, Fallback::Unit(fallback_ident) | Fallback::WithValue(fallback_ident) if variant_ident == fallback_ident)
    }
}

/// finds a single enum variant with the attribute "fallback".
/// a "fallback variant" may come in one of two forms:
/// 1. `#[fallback] Foo`, which we map to `Fallback::Unit`
/// 2. `#[fallback] Foo(uN)`, where `N` is the enum's bitsize and `Foo` is the enum's last variant, 
/// which we map to `Fallback::WithValue`
pub fn fallback_variant(data: &Data, enum_bitsize: BitSize) -> Option<Fallback> {
    match data {
        Data::Enum(enum_data) => {
            let variants_with_fallback = enum_data
                .variants
                .iter()
                .filter(|variant| variant.attrs.iter().any(is_fallback_attribute));

            match variants_with_fallback.at_most_one() {
                Ok(None) => None,
                Ok(Some(variant)) => {
                    let is_last_variant = variant.ident == enum_data.variants.last().unwrap().ident;
                    let fallback = Fallback::from_variant(variant, enum_bitsize, is_last_variant);
                    Some(fallback)
                },
                Err(_) => abort_call_site!("only one enum variant may be `#[fallback]`"; help = "remove #[fallback] attributes until you only have one"),
            }
        }
        Data::Struct(struct_data) => {
            let mut field_attrs = struct_data.fields.iter().flat_map(|field| &field.attrs);

            if field_attrs.any(is_fallback_attribute) {
                abort_call_site!("`#[fallback]` is only applicable to enums"; help = "remove all `#[fallback]` from this struct")
            } else {
                None
            }
        }
        _ => unreachable(()),
    }
}
