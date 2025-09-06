use proc_macro2::TokenStream;
use proc_macro_error2::abort_call_site;
use quote::quote;
use syn::{Data, Field, Fields};

use crate::shared::{self, unreachable};

fn filter_not_reserved_or_padding(field: &&Field) -> bool {
    let field_name_string = field.ident.as_ref().unwrap().to_string();
    !field_name_string.starts_with("reserved_") && !field_name_string.starts_with("padding_")
}

pub(super) fn json_schema_bits(item: TokenStream) -> TokenStream {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => abort_call_site!("use derive(JsonSchema) for enums"),
        Data::Union(_) => unreachable(()),
    };

    let json_schema_impl = match struct_data.fields {
        Fields::Named(fields) => {
            let calls = fields.named.iter().filter(filter_not_reserved_or_padding).map(|f| {
                // We can unwrap since this is a named field
                let field_name = f.ident.as_ref().unwrap().to_string();
                let ty = &f.ty;
                quote! {
                    object_validation
                        .properties
                        .insert(<str as ::std::borrow::ToOwned>::to_owned(#field_name), generator.subschema_for::<#ty>());
                    object_validation.required.insert(<str as ::std::borrow::ToOwned>::to_owned(#field_name));
                }
            });
            quote! {
                let mut schema_object = ::schemars::schema::SchemaObject {
                    instance_type: ::core::option::Option::Some(::schemars::schema::SingleOrVec::Single(::std::boxed::Box::new(::schemars::schema::InstanceType::Object))),
                    ..::core::default::Default::default()
                };
                let object_validation = schema_object.object();
                object_validation.additional_properties =
                    ::core::option::Option::Some(::std::boxed::Box::new(::schemars::schema::Schema::Bool(false)));
                #(#calls)*
                ::schemars::schema::Schema::Object(schema_object)
            }
        }
        Fields::Unnamed(fields) => {
            let len = fields.unnamed.len() as u32;
            let calls = fields.unnamed.iter().map(|f| {
                let ty = &f.ty;
                quote!(generator.subschema_for::<#ty>())
            });
            quote! {
                ::schemars::schema::Schema::Object(::schemars::schema::SchemaObject {
                    instance_type: ::core::option::Option::Some(::schemars::schema::SingleOrVec::Single(::std::boxed::Box::new(::schemars::schema::InstanceType::Array))),
                    array: ::core::option::Option::Some(::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                        items: ::core::option::Option::Some(::schemars::schema::SingleOrVec::Vec(::std::vec![#(#calls),*])),
                        max_items: ::core::option::Option::Some(#len),
                        min_items: ::core::option::Option::Some(#len),
                        ..::core::default::Default::default()
                    })),
                    ..::core::default::Default::default()
                })
            }
        }
        Fields::Unit => todo!("this is a unit struct, which is not supported right now"),
    };

    quote! {
        impl ::schemars::JsonSchema for #name {
            fn schema_name() -> ::std::string::String {
                <str as ::std::borrow::ToOwned>::to_owned(#name_str)
            }

            fn schema_id() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(concat!(module_path!(), "::", #name_str))
            }

            fn json_schema(generator: &mut ::schemars::r#gen::SchemaGenerator) -> ::schemars::schema::Schema {
                #json_schema_impl
            }
        }
    }
}
