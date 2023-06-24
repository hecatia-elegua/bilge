use super::{BitSize, is_fallback_attribute, unreachable};
use crate::bitsize::bitsize_from_type_token;
use crate::shared::util::{Single, SingleResult};
use proc_macro2::Ident;
use proc_macro_error::{abort, abort_call_site};
use syn::{Type, Variant, Data};

pub enum Fallback {
    Unit(Ident),
    WithValue(Ident),
}

impl Fallback {
    fn from_variant(variant: &Variant, enum_bitsize: BitSize) -> Fallback {
        use syn::Fields::*;

        let ident = variant.ident.to_owned();

        match &variant.fields {
            Named(_) => {
                abort!(variant, "fallback does not support variants with named fields"; help = "use a tuple variant or remove this fallback")
            }
            Unnamed(fields) => {
                let variant_fields = fields.unnamed.iter();

                let SingleResult::Single(fallback_value) = variant_fields.single() else {
                    abort!(variant, "fallback variant must have exactly one field"; help = "use only one field or change to a unit variant")
                };

                match &fallback_value.ty {
                    Type::Path(type_path) => match bitsize_from_type_token(&type_path.path) {
                        Some(bitsize) if bitsize == enum_bitsize => Fallback::WithValue(ident),
                        Some(bitsize) => abort!(
                            variant.fields,
                            "bitsize of fallback field ({}) does not match bitsize of enum ({})",
                            bitsize,
                            enum_bitsize
                        ),
                        None => abort!(
                            variant.fields,
                            "fallback only supports arbitrary_int or bool types"
                        ),
                    },
                    Type::Reference(_) => {
                        abort!(variant.fields, "fallback does not support references")
                    }
                    _ => abort!(
                        variant.fields,
                        "fallback only supports arbitrary_int or bool types"
                    ),
                }
            }
            Unit => Fallback::Unit(ident),
        }
    }
}

/// finds a single enum variant with the attribute "fallback".
/// a "fallback variant" may come in one of two forms:
/// 1. `#[fallback] Foo`, which we map to `Fallback::Unit`
/// 2. `#[fallback] Foo(uN)`, where `N` is the bitsize of the enum, which we map to `Fallback::WithValue`
pub fn fallback_variant(data: &Data, enum_bitsize: BitSize) -> Option<Fallback> {
    match data {
        Data::Enum(enum_data) => {
            let variants_with_fallback = enum_data
                .variants
                .iter()
                .filter(|variant| variant.attrs.iter().any(is_fallback_attribute));

            match variants_with_fallback.single() {
                SingleResult::Single(variant) => {
                    let fallback = Fallback::from_variant(variant, enum_bitsize);
                    Some(fallback)
                }
                SingleResult::MoreThanOne => abort_call_site!(
                    "only one enum variant may be fallback";
                    help = "remove #[fallback] attributes until you only have one"
                ),
                SingleResult::Empty => None,
            }
        }
        Data::Struct(struct_data) => {
            let mut field_attrs = struct_data.fields.iter().flat_map(|field| &field.attrs);

            if field_attrs.any(is_fallback_attribute) {
                abort_call_site!("the attribute `fallback` is only applicable to enums"; help = "remove all `#[fallback]` from this struct")
            } else {
                None
            }
        }
        _ => unreachable(()),
    }
}
