use proc_macro2::{TokenStream, Ident};
use proc_macro_error::abort_call_site;
use quote::quote;
use syn::{Data, Fields};

use crate::shared::{self, unreachable};

pub(super) fn debug_bits(item: TokenStream) -> TokenStream {
    let derive_input = shared::parse_derive(item);
    let name = &derive_input.ident;
    let name_str = name.to_string();
    let mut fieldless_next_int = 0;
    let struct_data = match derive_input.data {
        Data::Struct(s) => s,
        Data::Enum(_) => abort_call_site!("use derive(Debug) for enums"),
        Data::Union(_) => abort_call_site!("item is not a struct, try derive(Debug)"),
    };
    
    let fmt_impl = match struct_data.fields {
        Fields::Named(fields) => {
            let calls = fields.named.iter().map(|f| {
                // We can unwrap since this is a named field
                let call = f.ident.as_ref().unwrap();
                let name = call.to_string();
                quote!(.field(#name, &self.#call()))
            });
            quote! {
                f.debug_struct(#name_str)
                // .field("field1", &self.field1()).field("field2", &self.field2()).field("field3", &self.field3()).finish()
                #(#calls)*.finish()
            }
        },
        Fields::Unnamed(fields) => {
            let calls = fields.unnamed.iter().map(|_| {
                let call: Ident = syn::parse_str(&format!("val_{}", fieldless_next_int))
                    .unwrap_or_else(unreachable);
                fieldless_next_int += 1;
                quote!(.field(&self.#call()))
            });
            quote! {
                f.debug_tuple(#name_str)
                // .field(&self.val0()).field(&self.val1()).finish()
                #(#calls)*.finish()
            }
        },
        Fields::Unit => todo!("this is a unit struct, which is not supported right now"),
    };
    
    quote! {
        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                #fmt_impl
            }
        }
    }
}
