use manyhow::bail;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, Field, Fields};

use crate::shared::{self, unreachable};

fn filter_not_reserved_or_padding(field: &&Field) -> bool {
    let field_name_string = field.ident.as_ref().unwrap().to_string();
    !field_name_string.starts_with("reserved_") && !field_name_string.starts_with("padding_")
}

pub(super) fn json_schema_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => bail!("use derive(JsonSchema) for enums"),
        Data::Union(_) => unreachable(()),
    };

    let json_schema_impl = match struct_data.fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter().filter(filter_not_reserved_or_padding).collect::<Vec<_>>();
            let properties = fields.iter().map(|f| {
                // We can unwrap since this is a named field
                let field_name = f.ident.as_ref().unwrap().to_string();
                let ty = &f.ty;
                quote!(#field_name: generator.subschema_for::<#ty>())
            });
            let required = fields.iter().map(|f| {
                // We can unwrap since this is a named field
                let field_name = f.ident.as_ref().unwrap().to_string();
                quote!(#field_name)
            });

            if fields.is_empty() {
                quote! {
                    ::schemars::json_schema!({
                        "type": "object",
                        "additionalProperties": false,
                    })
                }
            } else {
                quote! {
                    ::schemars::json_schema!({
                        "type": "object",
                        "properties": { #(#properties),* },
                        "required": [#(#required),*],
                        "additionalProperties": false,
                    })
                }
            }
        }
        Fields::Unnamed(fields) => {
            let len = fields.unnamed.len() as u32;
            let calls = fields.unnamed.iter().map(|f| {
                let ty = &f.ty;
                quote!(generator.subschema_for::<#ty>())
            });
            quote! {
                ::schemars::json_schema!({
                    "type": "array",
                    "prefixItems": [#(#calls),*],
                    "maxItems": #len,
                    "minItems": #len,
                })
            }
        }
        Fields::Unit => bail!("unit structs are not supported"),
    };

    Ok(quote! {
        impl ::schemars::JsonSchema for #name {
            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(#name_str)
            }

            fn schema_id() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(concat!(module_path!(), "::", #name_str))
            }

            fn json_schema(generator: &mut ::schemars::SchemaGenerator) -> ::schemars::Schema {
                #json_schema_impl
            }
        }
    })
}
