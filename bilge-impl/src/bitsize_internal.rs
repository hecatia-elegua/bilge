use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, abort};
use quote::quote;
use syn::{Attribute, Field, Item, ItemEnum, ItemStruct, Type, Fields, Variant, punctuated::Iter};
use crate::shared::{self, unreachable, BitSize, MAX_ENUM_BIT_SIZE, is_fallback_attribute, enum_fills_bitsize};


pub(crate) mod struct_gen;

/// Intermediate Representation, just for bundling these together
struct ItemIr<'a> {
    attrs: &'a Vec<Attribute>,
    name: &'a Ident,
    /// needed in from_bits and try_from_bits
    filled_check: TokenStream,
    /// generated item (and setters, getters, constructor, impl Bitsized)
    expanded: TokenStream,
}

pub(super) fn bitsize_internal(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, arb_int, bitsize) = parse(item, args);
    let ir = match item {
        Item::Struct(ref item) => {
            let expanded = generate_struct(item, &arb_int);
            let filled_check = generate_struct_filled_check(&item.fields);
            let attrs = &item.attrs;
            let name = &item.ident;
            ItemIr { attrs, name, filled_check, expanded }
        }
        Item::Enum(ref item) => {
            let expanded = generate_enum(item);
            let filled_check = generate_enum_filled_check(bitsize, item.variants.iter());
            let attrs = &item.attrs;
            let name = &item.ident;
            ItemIr { attrs, name, filled_check, expanded }
        }
        _ => unreachable(()),
    };
    generate_common(ir, &arb_int)
}

fn parse(item: TokenStream, args: TokenStream) -> (Item, TokenStream, BitSize) {
    let item = syn::parse2(item).unwrap_or_else(unreachable);
    let (declared_bitsize, arb_int) = shared::bitsize_and_arbitrary_int_from(args);
    (item, arb_int, declared_bitsize)
}

fn generate_struct(struct_data: &ItemStruct, arb_int: &TokenStream) -> TokenStream {
    let ItemStruct { vis, ident, fields, .. } = struct_data;

    let mut fieldless_next_int = 0;
    let mut previous_field_sizes = vec![];
    let (accessors, (constructor_args, constructor_parts)): (Vec<TokenStream>, (Vec<TokenStream>, Vec<TokenStream>)) = fields.iter()
        .map(|field| {
            // offset is needed for bit-shifting
            // struct Example { field1: u8, field2: u4, field3: u4 }
            // previous_field_sizes = []     -> unwrap_or_else -> field_offset = 0
            // previous_field_sizes = [8]    -> reduce         -> field_offset = 0 + 8     =  8
            // previous_field_sizes = [8, 4] -> reduce         -> field_offset = 0 + 8 + 4 = 12
            let field_offset = previous_field_sizes.iter().cloned().reduce(|acc, next| quote!(#acc + #next)).unwrap_or_else(|| quote!(0));
            let field_size = shared::generate_type_bitsize(&field.ty);
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
        let size = shared::generate_type_bitsize(ty);
        return (quote!(#getter), (quote!(), quote! { {
            // we still need to shift by the element's size
            offset += #size;
            0
        } }))
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

    let getter_value = struct_gen::generate_getter_value(ty, offset, false);

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    let array_at = if let Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len_expr = &array.len;
        let name: Ident = syn::parse_str(&format!("{name}_at")).unwrap_or_else(unreachable);
        let getter_value = struct_gen::generate_getter_value(elem_ty, offset, true);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity)]
            #vis #const_ fn #name(&self, index: usize) -> #elem_ty {
                assert!(index < #len_expr);
                #getter_value
            }
        }
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
        
        #array_at
    }
}

fn generate_setter(field: &Field, offset: &TokenStream, name: &Ident) -> TokenStream {
    let Field { attrs, vis, ty, .. } = field;
    let setter_value = struct_gen::generate_setter_value(ty, offset, false);

    let name: Ident = syn::parse_str(&format!("set_{name}")).unwrap_or_else(unreachable);

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    let array_at = if let Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len_expr = &array.len;
        let name: Ident = syn::parse_str(&format!("{name}_at")).unwrap_or_else(unreachable);
        let setter_value = struct_gen::generate_setter_value(elem_ty, offset, true);
        quote! {
            // #[inline]
            #(#attrs)*
            #[allow(clippy::type_complexity)]
            #vis #const_ fn #name(&mut self, index: usize, value: #elem_ty) {
                assert!(index < #len_expr);
                #setter_value
            }
        }
    } else {
        quote!()
    };

    quote! {
        // #[inline]
        #(#attrs)*
        #[allow(clippy::type_complexity)]
        #vis #const_ fn #name(&mut self, value: #ty) {
            #setter_value
        }

        #array_at
    }
}

fn generate_constructor_stuff(ty: &Type, name: &Ident) -> (TokenStream, TokenStream) {
    let constructor_arg = quote! {
        #name: #ty,
    };
    let constructor_part = struct_gen::generate_constructor_part(ty, name);
    (constructor_arg, constructor_part)
}

fn generate_filled_check_for(ty: &Type) -> TokenStream {
    if shared::is_always_filled(ty) {
        return quote!(true);
    }
    use Type::*;
    match ty {
        // These don't work with structs or aren't useful in bitfields.
        BareFn(_) | Group(_) | ImplTrait(_) | Infer(_) | Macro(_) | Never(_) |
        // We could provide some info on error as to why Ptr/Reference won't work due to safety.
        Ptr(_) | Reference(_) |
        // The bitsize must be known at compile time.
        Slice(_) |
        // Something to investigate, but doesn't seem useful/usable here either.
        TraitObject(_) |
        // I have no idea where this is used.
        Verbatim(_) | Paren(_) => abort!(ty, "This field type is not supported"),
        Tuple(tuple) => {
            tuple.elems.iter().map(generate_filled_check_for)
                .reduce(|acc, next| quote!((#acc && #next)))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(true))
        },
        Array(array) => {
            generate_filled_check_for(&array.elem)
        },
        Path(_) => {
            quote!(<#ty as Bitsized>::FILLED)
        },
        _ => abort!(ty, "This field type is currently not supported"),
    }
}

fn generate_struct_filled_check(fields: &Fields) -> TokenStream {
    if fields.is_empty() {
        abort_call_site!("structs without fields are not supported")
    }

    // NEVER move this, since we validate all nested field types here as well.
    // If we do want to move this, add a new function just for validation.
    fields.iter()
        .map(|field| generate_filled_check_for(&field.ty))
        .reduce(|acc, next| quote!(#acc && #next))
        //when we only have uints or nothing as fields, return true
        .unwrap_or_else(|| quote!(true))
}

fn generate_enum_filled_check(bitsize: BitSize, variants: Iter<Variant>) -> TokenStream {
    let variant_count = variants.clone().count();
    if variant_count == 0 {
        abort_call_site!("empty enums are not supported");
    }

    if bitsize > MAX_ENUM_BIT_SIZE {
        abort_call_site!("enum bitsize is limited to {}", MAX_ENUM_BIT_SIZE)
    }
    
    let has_fallback = variants.flat_map(|variant| &variant.attrs).any(is_fallback_attribute);
    
    if has_fallback {
        quote!(true)
    } else {
        let enum_is_filled = enum_fills_bitsize(bitsize, variant_count);
        quote!(#enum_is_filled)    
    }
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
    let ItemIr { attrs, name, filled_check, expanded } = ir;

    quote! {
        #(#attrs)*
        #expanded
        impl bilge::Bitsized for #name {
            type ArbitraryInt = #arb_int;
            const BITS: usize = <Self::ArbitraryInt as Bitsized>::BITS;
            const MAX: Self::ArbitraryInt = <Self::ArbitraryInt as Bitsized>::MAX;
            const FILLED: bool = #filled_check;
        }
    }
}