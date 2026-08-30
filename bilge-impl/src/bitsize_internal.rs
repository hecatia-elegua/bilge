use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, Field, Item, ItemEnum, ItemStruct, Type, Visibility};

use crate::shared::{self, BitsizeArgs, attrs_without_at, place_struct_fields, unreachable};

pub(crate) mod struct_gen;

/// Intermediate Representation, just for bundling these together
struct ItemIr<'a> {
    attrs: &'a Vec<Attribute>,
    name: &'a Ident,
    /// generated item (and setters, getters, constructor, impl Bitsized)
    expanded: TokenStream,
}

pub(super) fn bitsize_internal(args: TokenStream, item: TokenStream) -> manyhow::Result {
    let (item, args) = parse(item, args)?;
    let ir = match item {
        Item::Struct(ref item) => {
            let expanded = generate_struct(item, &args.arb_int, &args.new_vis, args.bitsize)?;
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
    Ok(generate_common(ir, &args.arb_int))
}

fn parse(item: TokenStream, args: TokenStream) -> manyhow::Result<(Item, BitsizeArgs)> {
    let item = syn::parse2(item).unwrap_or_else(unreachable);
    let args = shared::parse_bitsize_args(args)?;
    Ok((item, args))
}

fn generate_struct(struct_data: &ItemStruct, arb_int: &TokenStream, new_vis: &Visibility, declared_bitsize: u8) -> manyhow::Result<TokenStream> {
    let ItemStruct { vis, ident, fields, .. } = struct_data;
    let layout = place_struct_fields(fields, declared_bitsize as usize)?;

    type TokenVec = Vec<TokenStream>;
    let (accessors, (constructor_args, (constructor_parts, shifted_names))): (TokenVec, (TokenVec, (TokenVec, Vec<Ident>))) = fields
        .iter()
        .zip(layout.fields.iter())
        .enumerate()
        .map(|(i, (field, place))| generate_field(field, &place.offset, i))
        .unzip();

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    Ok(quote! {
        #[repr(transparent)]
        #vis struct #ident {
            /// WARNING: modifying this value directly can break invariants.
            /// Use `#[bitsize(N, hide_value)]` to make this field inaccessible.
            value: #arb_int,
        }
        impl #ident {
            // #[inline]
            #[allow(clippy::too_many_arguments, clippy::type_complexity, missing_docs, unused_parens)]
            #new_vis #const_ fn new(#( #constructor_args )*) -> Self {
                type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
                type BaseIntOf<T> = <ArbIntOf<T> as Integer>::UnderlyingType;

                #( #constructor_parts )*
                let raw_value = #( #shifted_names )|*;
                let value = #arb_int::new(raw_value);
                Self { value }
            }
            #( #accessors )*
        }
    })
}

fn generate_field(field: &Field, field_offset: &TokenStream, i: usize) -> (TokenStream, (TokenStream, (TokenStream, Ident))) {
    let Field { ident, ty, .. } = field;
    let name = if let Some(ident) = ident {
        ident.clone()
    } else {
        let name = format!("val_{i}");
        syn::parse_str(&name).unwrap_or_else(unreachable)
    };

    // skip reserved fields in constructors and setters
    let name_str = name.to_string();
    if name_str.contains("reserved_") || name_str.contains("padding_") {
        // needed for `DebugBits`
        let getter = generate_getter(field, field_offset, &name);
        let accessors = quote!(#getter);
        let constructor_arg = quote!();
        let shifted_name = format!("shifted_{name}");
        let shifted_name: Ident = syn::parse_str(&shifted_name).unwrap_or_else(unreachable);
        // holes / reserved bits stay 0
        let constructor_part = quote! {
            let #shifted_name = 0;
        };
        return (accessors, (constructor_arg, (constructor_part, shifted_name)));
    }

    let getter = generate_getter(field, field_offset, &name);
    let setter = generate_setter(field, field_offset, &name);
    let toggle = generate_toggle(field, &name);
    let (constructor_arg, constructor_part, shifted_name) = generate_constructor_stuff(ty, &name, field_offset);

    let accessors = quote! {
        #getter
        #setter
        #toggle
    };

    (accessors, (constructor_arg, (constructor_part, shifted_name)))
}

fn generate_getter(field: &Field, offset: &TokenStream, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;
    let attrs = attrs_without_at(attrs);

    let getter_value = struct_gen::generate_getter_value(ty, offset, false);

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let array_at = if let Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len_expr = &array.len;
        let at_name = at_ident(name);
        let getter_value = struct_gen::generate_getter_value(elem_ty, offset, true);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity, unused_parens)]
            #vis #const_ fn #at_name(&self, index: usize) -> #elem_ty {
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
    let attrs = attrs_without_at(attrs);
    let setter_value = struct_gen::generate_setter_value(ty, offset, false);

    let setter_name = setter_ident(name);

    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let array_at = if let Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len_expr = &array.len;
        let setter_at = at_ident(&setter_name);
        let setter_value = struct_gen::generate_setter_value(elem_ty, offset, true);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity, unused_parens)]
            #vis #const_ fn #setter_at(&mut self, index: usize, value: #elem_ty) {
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
        #vis #const_ fn #setter_name(&mut self, value: #ty) {
            #setter_value
        }

        #array_at
    }
}

fn is_bool_type(ty: &Type) -> bool {
    shared::last_ident_of_path(ty).is_some_and(|ident| ident == "bool")
}

fn accessor_ident(prefix: &str, name: &Ident, suffix: &str) -> Ident {
    syn::parse_str(&format!("{prefix}{name}{suffix}")).unwrap_or_else(unreachable)
}

fn setter_ident(name: &Ident) -> Ident {
    accessor_ident("set_", name, "")
}

fn at_ident(name: &Ident) -> Ident {
    accessor_ident("", name, "_at")
}

fn toggle_ident(name: &Ident) -> Ident {
    accessor_ident("toggle_", name, "")
}

/// `toggle_foo` for `bool` fields, `toggle_foo_at` for `[bool; N]`.
/// Reserved/padding fields never reach here (no setter either).
fn generate_toggle(field: &Field, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;
    let attrs = attrs_without_at(attrs);
    let const_ = if cfg!(feature = "nightly") { quote!(const) } else { quote!() };

    let scalar = if is_bool_type(ty) {
        let toggle_name = toggle_ident(name);
        let setter_name = setter_ident(name);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity, unused_parens)]
            #vis #const_ fn #toggle_name(&mut self) {
                let next = !self.#name();
                self.#setter_name(next);
            }
        }
    } else {
        quote!()
    };

    let array_at = if let Type::Array(array) = ty {
        if is_bool_type(&array.elem) {
            let len_expr = &array.len;
            let at_name = at_ident(name);
            let setter_at = setter_ident(&at_name);
            let toggle_at = toggle_ident(&at_name);
            quote! {
                // #[inline]
                #(#attrs)*
                #[allow(clippy::type_complexity, unused_parens)]
                #vis #const_ fn #toggle_at(&mut self, index: usize) {
                    ::core::assert!(index < #len_expr);
                    let next = !self.#at_name(index);
                    self.#setter_at(index, next);
                }
            }
        } else {
            quote!()
        }
    } else {
        quote!()
    };

    quote! {
        #scalar
        #array_at
    }
}

fn generate_constructor_stuff(ty: &Type, name: &Ident, offset: &TokenStream) -> (TokenStream, TokenStream, Ident) {
    let name = format!("arg_{name}");
    let name: Ident = syn::parse_str(&name).unwrap_or_else(unreachable);
    let constructor_arg = quote! {
        #name: #ty,
    };
    let shifted_name = format!("shifted_{name}");
    let shifted_name: Ident = syn::parse_str(&shifted_name).unwrap_or_else(unreachable);

    let constructor_part = struct_gen::generate_constructor_part(ty, &name, &shifted_name, offset);
    (constructor_arg, constructor_part, shifted_name)
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
