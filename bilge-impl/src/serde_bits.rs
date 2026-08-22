use itertools::MultiUnzip;
use manyhow::bail;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Data, Field, Fields};

use crate::shared::{self, unreachable};

fn filter_not_reserved_or_padding(field: &&Field) -> bool {
    let field_name_string = field.ident.as_ref().unwrap().to_string();
    !field_name_string.starts_with("reserved_") && !field_name_string.starts_with("padding_")
}

pub(super) fn serialize_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => bail!("use derive(Serialize) for enums"),
        Data::Union(_) => unreachable(()),
    };

    let serialize_impl = match struct_data.fields {
        Fields::Named(fields) => {
            let struct_calls = fields.named.iter().filter(filter_not_reserved_or_padding).map(|f| {
                // We can unwrap since this is a named field
                let call = f.ident.as_ref().unwrap();
                let name = call.to_string();
                quote!(state.serialize_field(#name, &self.#call())?;)
            });
            let map_calls = fields.named.iter().filter(filter_not_reserved_or_padding).map(|f| {
                // We can unwrap since this is a named field
                let call = f.ident.as_ref().unwrap();
                let name = call.to_string();
                quote!(state.serialize_entry(#name, &self.#call())?;)
            });
            let len = fields.named.iter().filter(filter_not_reserved_or_padding).count();
            quote! {
                if serializer.is_human_readable() {
                    use ::serde::ser::SerializeMap;
                    let mut state = serializer.serialize_map(::core::option::Option::Some(#len))?;
                    #(#map_calls)*
                    state.end()
                } else {
                    use ::serde::ser::SerializeStruct;
                    let mut state = serializer.serialize_struct(#name_str, #len)?;
                    #(#struct_calls)*
                    state.end()
                }
            }
        }
        Fields::Unnamed(fields) => {
            let calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                let call = format_ident!("val_{i}");
                quote!(state.serialize_field(&self.#call())?;)
            });
            let len = fields.unnamed.len();
            quote! {
                use ::serde::ser::SerializeTupleStruct;
                let mut state = serializer.serialize_tuple_struct(#name_str, #len)?;
                // state.serialize_field(&self.val0())?; state.serialize_field(&self.val1())?; state.end()
                #(#calls)*
                state.end()
            }
        }
        Fields::Unit => bail!("unit structs are not supported"),
    };

    Ok(quote! {
        impl ::serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                #serialize_impl
            }
        }
    })
}

fn deserialize_field_parts(
    i: usize, field_ident: &Ident,
) -> (
    TokenStream,
    TokenStream,
    TokenStream,
    TokenStream,
    TokenStream,
    TokenStream,
    TokenStream,
    String,
) {
    let field_name_string = field_ident.to_string();
    (
        quote!(#field_ident,),
        quote!(#field_name_string => Ok(Field::#field_ident),),
        quote!(#field_name_string,),
        quote!(let #field_ident = seq.next_element()?.ok_or_else(|| ::serde::de::Error::invalid_length(#i, &self))?;),
        quote!(let mut #field_ident = None;),
        quote!(Field::#field_ident => {
            if #field_ident.is_some() {
                return Err(::serde::de::Error::duplicate_field(#field_name_string));
            }
            #field_ident = Some(map.next_value()?);
        }),
        quote!(let #field_ident = #field_ident.ok_or_else(|| ::serde::de::Error::missing_field(#field_name_string))?;),
        format!("`{}`", field_name_string),
    )
}

pub(super) fn deserialize_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let struct_name_str = format!("struct {}", name_str);
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => bail!("use derive(Deserialize) for enums"),
        Data::Union(_) => unreachable(()),
    };

    let should_have_visit_map = matches!(struct_data.fields, Fields::Named(_));

    let (
        field_names,
        field_deserialize,
        field_name_strings,
        field_visit_seq,
        field_visit_map_init,
        field_visit_map_match,
        field_visit_map_check,
        mut field_expecting,
    ): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>) = match struct_data.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(filter_not_reserved_or_padding)
            .enumerate()
            .map(|(i, f)| deserialize_field_parts(i, f.ident.as_ref().unwrap()))
            .multiunzip(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, _)| deserialize_field_parts(i, &format_ident!("val_{i}")))
            .multiunzip(),
        Fields::Unit => bail!("unit structs are not supported"),
    };

    if field_expecting.len() > 1 {
        field_expecting.last_mut().unwrap().insert_str(0, "or ");
    }
    let field_expecting = field_expecting.join(", ");

    let visit_map = if should_have_visit_map {
        quote!(fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
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
        })
    } else {
        quote!()
    };

    let visit_seq = quote!(
        fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
        where
            V: ::serde::de::SeqAccess<'de>,
        {
            #(#field_visit_seq)*
            Ok(Self::Value::new(#(#field_names)*))
        }
    );

    let deserialize = if should_have_visit_map {
        quote!(
            if deserializer.is_human_readable() {
                deserializer.deserialize_map(Visitor)
            } else {
                deserializer.deserialize_struct(#name_str, FIELDS, Visitor)
            }
        )
    } else {
        let field_count = field_names.len();
        quote!(deserializer.deserialize_tuple_struct(#name_str, #field_count, Visitor))
    };

    Ok(quote! {
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

                    #visit_seq
                    #visit_map
                }

                const FIELDS: &'static [&'static str] = &[#(#field_name_strings)*];
                #deserialize
            }
        }
    })
}
