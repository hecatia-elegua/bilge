use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, Field, Fields, Item, ItemEnum, ItemStruct, Type, Variant, Visibility, punctuated::Iter};

use crate::shared::discriminant::EnumDiscriminant;
use crate::shared::{
    self, BitSize, BitsizeArgs, attrs_without_at, bitsize_from_type_ident, discriminant, discriminant_assigner::DiscriminantAssigner,
    discriminant_at, last_ident_of_path, parse_enum_discriminant, place_struct_fields, unreachable,
};

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
    let as_int = generate_as_int(&item, &args.arb_int, args.bitsize)?;
    let ir = match &item {
        Item::Struct(item) => {
            let expanded = generate_struct(item, &args.arb_int, &args.new_vis, args.bitsize)?;
            ItemIr {
                attrs: &item.attrs,
                name: &item.ident,
                expanded,
            }
        }
        Item::Enum(item) => {
            let expanded = generate_enum(item);
            ItemIr {
                attrs: &item.attrs,
                name: &item.ident,
                expanded,
            }
        }
        _ => unreachable(()),
    };
    Ok(generate_common(ir, &args.arb_int, as_int))
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
    if shared::is_reserved_or_padding(&name_str) {
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
fn generate_common(ir: ItemIr, arb_int: &TokenStream, as_int: TokenStream) -> TokenStream {
    let ItemIr { attrs, name, expanded } = ir;

    quote! {
        #(#attrs)*
        #expanded
        impl ::bilge::Bitsized for #name {
            type ArbitraryInt = #arb_int;
            const BITS: usize = <Self::ArbitraryInt as Bitsized>::BITS;
            const MAX: Self::ArbitraryInt = <Self::ArbitraryInt as Bitsized>::MAX;
            #[inline]
            fn as_int(&self) -> Self::ArbitraryInt {
                #as_int
            }
        }
    }
}

fn generate_as_int(item: &Item, arb_int: &TokenStream, bitsize: BitSize) -> manyhow::Result<TokenStream> {
    match item {
        Item::Struct(_) => Ok(quote! { self.value }),
        Item::Enum(item) => generate_enum_as_int(item, arb_int, bitsize),
        _ => unreachable(()),
    }
}

fn generate_enum_as_int(item: &ItemEnum, arb_int: &TokenStream, bitsize: BitSize) -> manyhow::Result<TokenStream> {
    let name = &item.ident;

    let arms = match parse_enum_discriminant(&item.attrs)? {
        Some(EnumDiscriminant::Type(_)) => {
            let mut arms = Vec::new();
            for variant in &item.variants {
                let payload_ty = discriminant_at::variant_payload_ty(variant)?;
                arms.push(discriminant::payload_only_to_int_arm(name, &variant.ident, payload_ty, arb_int));
            }
            arms
        }
        tag => {
            let disc = match tag {
                Some(EnumDiscriminant::At(disc)) => {
                    disc.validate(bitsize as usize)?;
                    Some(disc)
                }
                _ => None,
            };
            generate_to_int_match_arms(item.variants.iter(), name, bitsize, arb_int, disc.as_ref())?
        }
    };

    Ok(quote! {
        match self {
            #(#arms)*
        }
    })
}

fn generate_to_int_match_arms(
    variants: Iter<Variant>, enum_name: &Ident, bitsize: BitSize, arb_int: &TokenStream, disc: Option<&discriminant_at::DiscriminantAt>,
) -> manyhow::Result<Vec<TokenStream>> {
    let fill_width = disc.map(|d| d.width as u8).unwrap_or(bitsize);
    let mut assigner = DiscriminantAssigner::new(fill_width);

    variants
        .map(|variant| -> manyhow::Result<TokenStream> {
            let variant_name = &variant.ident;
            let variant_value = assigner.assign_unsuffixed(variant)?;

            if let Some(disc) = disc {
                let payload_ty = discriminant_at::variant_payload_ty(variant)?;
                return Ok(discriminant_at::payload_to_int_arm_ref(
                    disc,
                    bitsize as usize,
                    enum_name,
                    variant_name,
                    payload_ty,
                    &variant_value,
                    arb_int,
                ));
            }

            Ok(match &variant.fields {
                Fields::Unit => shared::to_int_match_arm(enum_name, variant_name, arb_int, variant_value),
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let ty = &fields.unnamed[0].ty;
                    let same_width = last_ident_of_path(ty).and_then(bitsize_from_type_ident) == Some(bitsize);
                    if same_width {
                        quote! {
                            #enum_name::#variant_name(payload) => {
                                use ::bilge::Bitsized as _;
                                payload.as_int()
                            }
                        }
                    } else {
                        quote! { #enum_name::#variant_name(_) => #arb_int::new(#variant_value), }
                    }
                }
                Fields::Unnamed(_) => quote! { #enum_name::#variant_name(..) => #arb_int::new(#variant_value), },
                Fields::Named(_) => quote! { #enum_name::#variant_name { .. } => #arb_int::new(#variant_value), },
            })
        })
        .collect()
}
