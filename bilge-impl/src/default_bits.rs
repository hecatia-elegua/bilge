use manyhow::bail;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Type};

use crate::bitsize_internal::struct_gen;
use crate::shared::{self, parse_field_default, place_struct_fields, unreachable};

pub(crate) fn default_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = parse(item);
    let (derive_data, _, name, bitsize, ..) = shared::analyze_derive(&derive_input, crate::shared::DeriveKind::Other)?;

    match derive_data {
        Data::Struct(data) => generate_struct_default_impl(name, &data.fields, bitsize),
        Data::Enum(_) => bail!("use derive(Default) for enums"),
        _ => unreachable(()),
    }
}

fn generate_struct_default_impl(struct_name: &Ident, fields: &Fields, declared_bitsize: shared::BitSize) -> manyhow::Result<TokenStream> {
    let layout = place_struct_fields(fields, declared_bitsize as usize).unwrap_or_else(|_| unreachable(()));
    let mut parts = Vec::new();
    let mut needs_ctor_aliases = false;
    for (i, (field, place)) in fields.iter().zip(layout.fields.iter()).enumerate() {
        let offset = &place.offset;
        let default = parse_field_default(field)?;
        let name = field.ident.clone().unwrap_or_else(|| format_ident!("val_{i}"));
        let name_str = name.to_string();
        shared::reject_default_on_reserved(field, &name_str, default.is_some(), true)?;

        let part = if let Some(expr) = default {
            needs_ctor_aliases = true;
            let shifted_name = format_ident!("{name}_shifted");
            let ctor = struct_gen::generate_constructor_part(&field.ty, &name, &shifted_name, offset);
            quote! {{
                let #name = #expr;
                #ctor
                #shifted_name
            }}
        } else {
            let inner = generate_default_inner(&field.ty);
            quote! {{
                let mut offset = #offset;
                let shifted = #inner;
                shifted
            }}
        };
        parts.push(part);
    }
    let default_value = parts.into_iter().reduce(|acc, next| quote!(#acc | #next));
    let aliases = if needs_ctor_aliases {
        quote! {
            type ArbIntOf<T> = <T as ::bilge::Bitsized>::ArbitraryInt;
            type BaseIntOf<T> = <ArbIntOf<T> as ::bilge::arbitrary_int::traits::Integer>::UnderlyingType;
        }
    } else {
        quote!()
    };

    let import = shared::import_traits();
    Ok(quote! {
        impl ::core::default::Default for #struct_name {
            fn default() -> Self {
                #import
                #aliases
                let value = #default_value;
                let value = <#struct_name as ::bilge::Bitsized>::ArbitraryInt::new(value);
                Self { value }
            }
        }
    })
}

fn generate_default_inner(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        // TODO?: we could optimize nested arrays here like in `struct_gen.rs`
        // NOTE: in std, Default is only derived for arrays with up to 32 elements, but we allow more
        Array(array) => {
            let len_expr = &array.len;
            let elem_ty = &*array.elem;
            // generate the default value code for one array element
            let value_shifted = generate_default_inner(elem_ty);
            quote! {{
                // constness: iter, array::from_fn, for-loop, range are not const, so we're using while loops
                let mut acc = 0;
                let mut i = 0;
                while i < #len_expr {
                    // for every element, shift its value into its place
                    let value_shifted = #value_shifted;
                    // and bit-or them together
                    acc |= value_shifted;
                    i += 1;
                }
                acc
            }}
        }
        Path(path) => {
            let field_size = shared::generate_type_bitsize(ty);
            // u2::from(HaveFun::default()).value() as u32;
            quote! {{
                let as_int = <#path as ::bilge::Bitsized>::ArbitraryInt::from(<#path as ::core::default::Default>::default()).value();
                let as_base_int = as_int as <<Self as ::bilge::Bitsized>::ArbitraryInt as ::bilge::arbitrary_int::traits::Integer>::UnderlyingType;
                let shifted = as_base_int << offset;
                offset += #field_size;
                shifted
            }}
        }
        Tuple(tuple) => {
            tuple
                .elems
                .iter()
                .map(generate_default_inner)
                .reduce(|acc, next| quote!(#acc | #next))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        }
        _ => unreachable(()),
    }
}

fn parse(item: TokenStream) -> DeriveInput {
    shared::parse_derive(item)
}
