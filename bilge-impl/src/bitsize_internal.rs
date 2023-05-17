use proc_macro2::{TokenStream, Ident};
use quote::quote;
use syn::{Item, ItemStruct, ItemEnum, Type, Attribute, Field};

use crate::shared::{self, unreachable};

pub(crate) mod struct_gen;

/// Intermediate Representation, just for bundling these together
struct ItemIr<'a> {
    attrs: &'a Vec<Attribute>,
    name: &'a Ident,
    /// generated item (and setters, getters, constructor, impl Bitsized)
    expanded: TokenStream,
}

pub(super) fn bitsize_internal(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, arb_int) = parse(item, args);
    let ir = match item {
        Item::Struct(ref item) => {
            let expanded = generate_struct(item, &arb_int);
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

fn parse(item: TokenStream, args: TokenStream) -> (Item, TokenStream) {
    let item = syn::parse2(item).unwrap_or_else(unreachable);
    let (_declared_bitsize, arb_int) = shared::bitsize_and_arbitrary_int_from(args);
    (item, arb_int)
}

fn generate_struct(struct_data: &ItemStruct, arb_int: &TokenStream) -> TokenStream {
    let ItemStruct { vis, ident, fields, .. } = struct_data;

    let mut fieldless_next_int = 0;
    let mut previous_field_sizes = vec![];
    let (accessors, (constructor_args, constructor_parts)): (Vec<TokenStream>, (Vec<TokenStream>, Vec<TokenStream>)) = fields.iter()
        .map(|field| {
            let field_size = shared::generate_field_bitsize(&field.ty);
            // offset is needed for bit-shifting
            // struct Example { field1: u8, field2: u4, field3: u4 }
            // previous_field_sizes = []     -> unwrap_or_else -> field_offset = 0
            // previous_field_sizes = [8]    -> reduce         -> field_offset = 0 + 8     =  8
            // previous_field_sizes = [8, 4] -> reduce         -> field_offset = 0 + 8 + 4 = 12
            let field_offset = previous_field_sizes.iter().cloned().reduce(|acc, next| quote!(#acc + #next)).unwrap_or_else(|| quote!(0));
            previous_field_sizes.push(field_size);
            generate_field(field, &field_offset, &mut fieldless_next_int)
    }).unzip();

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    quote! {
        #vis struct #ident {
            /// WARNING: modifying this value directly can break invariants
            value: #arb_int,
        }
        impl #ident {
            // #[inline]
            #[allow(clippy::too_many_arguments, clippy::type_complexity)]
            pub #const_ fn new(#( #constructor_args )*) -> Self {
                type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
                type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;

                let mut offset = 0;
                let raw_value = #( #constructor_parts )|*;
                let value = #arb_int::new(raw_value);
                Self { value }
            }
            #( #accessors )*
        }
    }
}

fn generate_field(field: &Field, field_offset: &TokenStream, fieldless_next_int: &mut usize) -> (TokenStream, (TokenStream, TokenStream)) {
    let Field { ident, ty, .. } = field;
    let name = if let Some(ident) = ident {
        ident.clone()
    } else {
        let name = format!("val_{fieldless_next_int}");
        *fieldless_next_int += 1;
        syn::parse_str(&name).unwrap_or_else(unreachable)
    };

    // skip reserved fields in constructors and setters
    let name_str = name.to_string();
    if name_str.contains("reserved_") || name_str.contains("padding_") {
        // needed for `DebugBits`
        let getter = generate_getter(field, field_offset, &name);
        return (quote!(#getter), (quote!(), quote!(0)))
    }

    let getter = generate_getter(field, field_offset, &name);
    let setter = generate_setter(field, field_offset, &name);
    let (constructor_arg, constructor_part) = generate_constructor_stuff(ty, &name);

    let accessors = quote! {
        #getter
        #setter
    };
    
    (accessors, (constructor_arg, constructor_part))
}

fn generate_getter(field: &Field, offset: &TokenStream, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;

    let getter_value = struct_gen::generate_getter_value(ty, offset);

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    quote! {
        // #[inline]
        #(#attrs)*
        #[allow(clippy::type_complexity)]
        #vis #const_ fn #name(&self) -> #ty {
            #getter_value
        }
    }
}

fn generate_setter(field: &Field, offset: &TokenStream, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;
    let setter_value = struct_gen::generate_setter_value(ty, offset);

    let setter_name: Ident = syn::parse_str(&format!("set_{}", name)).unwrap_or_else(unreachable);

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    quote! {
        // #[inline]
        #(#attrs)*
        #[allow(clippy::type_complexity)]
        #vis #const_ fn #setter_name(&mut self, value: #ty) {
            #setter_value
        }
    }
}

fn generate_constructor_stuff(ty: &Type, name: &Ident) -> (TokenStream, TokenStream) {
    let constructor_arg = quote! {
        #name: #ty,
    };
    let constructor_part = struct_gen::generate_constructor_part(ty, name);
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
        impl bilge::Bitsized for #name {
            type ArbitraryInt = #arb_int;
            const BITS: usize = <Self::ArbitraryInt as Bitsized>::BITS;
            const MAX: Self::ArbitraryInt = <Self::ArbitraryInt as Bitsized>::MAX;
        }
    }
}
