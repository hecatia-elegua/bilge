use proc_macro2::{Ident, TokenStream};
use proc_macro_error::abort;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Attribute, Field, Fields, Item, ItemEnum, ItemStruct, Type};

use crate::shared::{self, unreachable, FieldLayout};

pub(crate) mod struct_gen;

/// Intermediate Representation, just for bundling these together
struct ItemIr<'a> {
    attrs: &'a Vec<Attribute>,
    name: &'a Ident,
    /// generated item (and setters, getters, constructor, impl Bitsized)
    expanded: TokenStream,
}

pub(super) fn bitsize_internal(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, arb_int, layout) = parse(item, args);
    let ir = match item {
        Item::Struct(ref item) => {
            let expanded = generate_struct(item, &arb_int, layout);
            let attrs = &item.attrs;
            let name = &item.ident;
            ItemIr { attrs, name, expanded }
        }
        Item::Enum(ref item) => {
            let expanded = generate_enum(item);
            let attrs = &item.attrs;
            let name = &item.ident;
            ItemIr { attrs, name, expanded }
        }
        _ => unreachable(()),
    };
    generate_common(ir, &arb_int)
}

fn parse(item: TokenStream, args: TokenStream) -> (Item, TokenStream, FieldLayout) {
    let item = syn::parse2(item).unwrap_or_else(unreachable);
    let (_declared_bitsize, arb_int, layout) = shared::bitsize_and_arbitrary_int_from(args);
    (item, arb_int, layout)
}

fn generate_struct(struct_data: &ItemStruct, arb_int: &TokenStream, layout: FieldLayout) -> TokenStream {
    let ItemStruct { vis, ident, fields, .. } = struct_data;

    let field_offsets = match layout {
        FieldLayout::Auto => generate_field_offsets_auto(fields),
        FieldLayout::Manual => generate_field_offsets_manual(fields),
    };

    let field_names = generate_field_names(fields);
    let (accessors, (constructor_args, constructor_parts)): (Vec<TokenStream>, (Vec<TokenStream>, Vec<TokenStream>)) = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| generate_field(field, &field_offsets[idx], &field_names[idx]))
        .unzip();

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    quote! {
        #vis struct #ident {
            /// WARNING: modifying this value directly can break invariants
            value: #arb_int,
        }
        impl #ident {
            // #[inline]
            #[allow(clippy::too_many_arguments, clippy::type_complexity, unused_parens)]
            pub #const_ fn new(#( #constructor_args )*) -> Self {
                type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
                type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;

                let raw_value = #( #constructor_parts )|*;
                let value = #arb_int::new(raw_value);
                Self { value }
            }
            #( #accessors )*
        }
    }
}

fn generate_field(field: &Field, field_offset: &TokenStream, name: &Ident) -> (TokenStream, (TokenStream, TokenStream)) {
    let Field { ty, .. } = field;

    // skip reserved fields in constructors and setters
    let name_str = name.to_string();
    if name_str.contains("reserved_") || name_str.contains("padding_") {
        // needed for `DebugBits`
        let getter = generate_getter(field, field_offset, name);
        let accessors = quote!(#getter);
        let constructor_arg = quote!();
        let constructor_part = quote! { {
            // reserved
            0
        } };
        return (accessors, (constructor_arg, constructor_part));
    }

    let getter = generate_getter(field, field_offset, &name);
    let setter = generate_setter(field, field_offset, &name);
    let (constructor_arg, constructor_part) = generate_constructor_stuff(ty, &name, field_offset);

    let accessors = quote! {
        #getter
        #setter
    };

    (accessors, (constructor_arg, constructor_part))
}

fn generate_getter(field: &Field, offset: &TokenStream, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;
    let attrs = remove_bit_attr(attrs);

    let getter_value = struct_gen::generate_getter_value(ty, offset, false);

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let array_at = if let Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len_expr = &array.len;
        let name: Ident = syn::parse_str(&format!("{name}_at")).unwrap_or_else(unreachable);
        let getter_value = struct_gen::generate_getter_value(elem_ty, offset, true);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity, unused_parens)]
            #vis #const_ fn #name(&self, index: usize) -> #elem_ty {
                ::core::assert!(index < #len_expr);
                #getter_value
            }
        }
    } else {
        quote!()
    };

    quote! {
        // #[inline]
        #(#attrs)*
        #[allow(clippy::type_complexity, unused_parens)]
        #vis #const_ fn #name(&self) -> #ty {
            #getter_value
        }

        #array_at
    }
}

fn generate_setter(field: &Field, offset: &TokenStream, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;
    let attrs = remove_bit_attr(attrs);
    let setter_value = struct_gen::generate_setter_value(ty, offset, false);

    let name: Ident = syn::parse_str(&format!("set_{name}")).unwrap_or_else(unreachable);

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let array_at = if let Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len_expr = &array.len;
        let name: Ident = syn::parse_str(&format!("{name}_at")).unwrap_or_else(unreachable);
        let setter_value = struct_gen::generate_setter_value(elem_ty, offset, true);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity, unused_parens)]
            #vis #const_ fn #name(&mut self, index: usize, value: #elem_ty) {
                ::core::assert!(index < #len_expr);
                #setter_value
            }
        }
    } else {
        quote!()
    };

    quote! {
        // #[inline]
        #(#attrs)*
        #[allow(clippy::type_complexity, unused_parens)]
        #vis #const_ fn #name(&mut self, value: #ty) {
            #setter_value
        }

        #array_at
    }
}

fn generate_constructor_stuff(ty: &Type, name: &Ident, offset: &TokenStream) -> (TokenStream, TokenStream) {
    let constructor_arg = quote! {
        #name: #ty,
    };
    let constructor_part = struct_gen::generate_constructor_part(ty, name, offset);
    (constructor_arg, constructor_part)
}

fn generate_enum(enum_data: &ItemEnum) -> TokenStream {
    let ItemEnum { vis, ident, variants, .. } = enum_data;
    quote! {
        #vis enum #ident {
            #variants
        }
    }
}

/// We have _one_ `generate_common` function, which holds everything struct and enum have _in common_.
/// Everything else has its own `generate_` functions.
fn generate_common(ir: ItemIr, arb_int: &TokenStream) -> TokenStream {
    let ItemIr { attrs, name, expanded } = ir;

    quote! {
        #(#attrs)*
        #expanded
        impl ::bilge::Bitsized for #name {
            type ArbitraryInt = #arb_int;
            const BITS: usize = <Self::ArbitraryInt as Bitsized>::BITS;
            const MAX: Self::ArbitraryInt = <Self::ArbitraryInt as Bitsized>::MAX;
        }
    }
}

fn generate_field_offsets_auto(fields: &Fields) -> Vec<TokenStream> {
    // For auto field layout, offset of field = (offset + bit size) of previous field
    fields.iter().take(fields.len() - 1).fold(vec![quote!(0)], |mut acc, f| {
        let prev = acc.last().unwrap();
        let bits = shared::generate_type_bitsize(&f.ty);
        acc.push(quote!(#prev + #bits));
        acc
    })
}

fn generate_field_offsets_manual(fields: &Fields) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let Some(attr) = shared::find_bit_range_attr(&f.attrs) else {
                abort!(f.span(), "required #[bit]/#[bits] attribute under manual layout")
            };
            let range = attr.parse().unwrap_or_else(|e| abort!(e.span(), e));
            let offset = range.start_bit;
            quote!(#offset)
        })
        .collect()
}

fn generate_field_names(fields: &Fields) -> Vec<Ident> {
    match fields {
        Fields::Named(fields) => fields.named.iter().map(|f| f.ident.clone().unwrap()).collect(),
        Fields::Unnamed(fields) => fields.unnamed.iter().enumerate().map(|(idx, _)| format_ident!("val_{idx}")).collect(),
        Fields::Unit => vec![],
    }
}

fn remove_bit_attr(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attr| !(attr.meta.path().is_ident("bit") || attr.meta.path().is_ident("bits")))
        .collect()
}
