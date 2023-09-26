use itertools::multiunzip;
use proc_macro2::{Ident, TokenStream};
use proc_macro_error::abort_call_site;
use quote::quote;
use syn::{Data, Field, Fields};

use crate::shared::{self, unreachable};

fn filter_not_reserved_or_padding(field: &&Field) -> bool {
    let field_name_string = field.ident.as_ref().unwrap().to_string();
    !field_name_string.starts_with("reserved_") && !field_name_string.starts_with("padding_")
}

pub(super) fn serialize_bits(item: TokenStream) -> TokenStream {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => abort_call_site!("use derive(Serialize) for enums"),
        Data::Union(_) => unreachable(()),
    };

    let serialize_impl = match struct_data.fields {
        Fields::Named(fields) => {
            let calls = fields.named.iter()
                .filter(filter_not_reserved_or_padding).map(|f| {
                // We can unwrap since this is a named field
                let call = f.ident.as_ref().unwrap();
                let name = call.to_string();
                quote!(state.serialize_field(#name, &self.#call())?;)
            });
            let len = fields.named.iter().filter(filter_not_reserved_or_padding).count();
            quote! {
                use ::serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct(#name_str, #len)?;
                // state.serialize_field("field1", &self.field1())?; state.serialize_field("field2", &self.field2())?; state.serialize_field("field3", &self.field3())?; state.end()
                #(#calls)*
                state.end()
            }
        }
        Fields::Unnamed(fields) => {
            let calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                let call: Ident = syn::parse_str(&format!("val_{}", i)).unwrap_or_else(unreachable);
                quote!(state.serialize_field(&self.#call())?;)
            });
            let len = fields.unnamed.len();
            quote! {
                use serde::ser::SerializeTupleStruct;
                let mut state = serializer.serialize_tuple_struct(#name_str, #len)?;
                // state.serialize_field(&self.val0())?; state.serialize_field(&self.val1())?; state.end()
                #(#calls)*
                state.end()
            }
        }
        Fields::Unit => todo!("this is a unit struct, which is not supported right now"),
    };

    quote! {
        impl ::serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                #serialize_impl
            }
        }
    }
}

fn deserialize_field_parts(i: usize, field_ident: &Ident) -> (TokenStream, TokenStream, TokenStream, TokenStream, TokenStream, TokenStream, TokenStream, String) {
    let field_name_string = field_ident.to_string();
    (
        quote!(#field_ident,)
    ,
        quote!(#field_name_string => Ok(Field::#field_ident),)
    ,
        quote!(#field_name_string,)
    ,
        quote!(let #field_ident = seq.next_element()?.ok_or_else(|| ::serde::de::Error::invalid_length(#i, &self))?;)
    ,
        quote!(let mut #field_ident = None;)
    ,
        quote!(Field::#field_ident => {
                    if #field_ident.is_some() {
                        return Err(::serde::de::Error::duplicate_field(#field_name_string));
                    }
                    #field_ident = Some(map.next_value()?);
                })
    ,
        quote!(let #field_ident = #field_ident.ok_or_else(|| ::serde::de::Error::missing_field(#field_name_string))?;)
    ,
        format!("`{}`", field_name_string)
    )
}

pub(super) fn deserialize_bits(item: TokenStream) -> TokenStream {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let struct_name_str = format!("struct {}", name_str);
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => abort_call_site!("use derive(Serialize) for enums"),
        Data::Union(_) => unreachable(()),
    };

    let (field_names, field_deserialize, field_name_strings, field_visit_seq, field_visit_map_init, field_visit_map_match, field_visit_map_check, mut field_expecting): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>) = match struct_data.fields {
        Fields::Named(fields) => {
            multiunzip(fields.named.iter()
                .filter(filter_not_reserved_or_padding).enumerate().map(|(i, f)| deserialize_field_parts(i, f.ident.as_ref().unwrap())))
        }
        Fields::Unnamed(fields) => {
            multiunzip(fields.unnamed.iter()
                .enumerate().map(|(i, _)| deserialize_field_parts(i, &syn::parse_str(&format!("val_{}", i)).unwrap_or_else(unreachable))))
        }
        Fields::Unit => todo!("this is a unit struct, which is not supported right now"),
    };

    if field_expecting.len() > 1 {
        field_expecting.last_mut().unwrap().insert_str(0, "or ");
    }
    let field_expecting = field_expecting.join(", ");

    quote! {
        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum Field { #(#field_names)* }
                impl<'de> ::serde::Deserialize<'de> for Field {
                    fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                    where
                        D: ::serde::Deserializer<'de>,
                    {
                        struct FieldVisitor;

                        impl<'de> ::serde::de::Visitor<'de> for FieldVisitor {
                            type Value = Field;

                            fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                formatter.write_str(#field_expecting)
                            }

                            fn visit_str<E>(self, value: &str) -> Result<Field, E>
                            where
                                E: ::serde::de::Error,
                            {
                                match value {
                                    #(#field_deserialize)*
                                    _ => Err(::serde::de::Error::unknown_field(value, FIELDS)),
                                }
                            }
                        }

                        deserializer.deserialize_identifier(FieldVisitor)
                    }
                }

                struct Visitor;

                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = #name;

                    fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        formatter.write_str(#struct_name_str)
                    }

                    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
                    where
                        V: ::serde::de::SeqAccess<'de>,
                    {
                        #(#field_visit_seq)*
                        Ok(Self::Value::new(#(#field_names)*))
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                    where
                        V: ::serde::de::MapAccess<'de>,
                    {
                        #(#field_visit_map_init)*
                        while let Some(key) = map.next_key()? {
                            match key {
                                #(#field_visit_map_match)*
                            }
                        }
                        #(#field_visit_map_check)*
                        Ok(#name::new(#(#field_names)*))
                    }
                }

                const FIELDS: &'static [&'static str] = &[#(#field_name_strings)*];
                deserializer.deserialize_struct(#name_str, FIELDS, Visitor)
            }
        }
    }
}
