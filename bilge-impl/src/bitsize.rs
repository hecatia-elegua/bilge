mod split;

use manyhow::bail;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use split::SplitAttributes;
use syn::{Fields, Item, ItemEnum, ItemStruct, Type, Variant, Visibility, parse_quote, punctuated::Iter, spanned::Spanned};

use crate::shared::{
    self, BitSize, BitsizeArgs, MAX_ENUM_BIT_SIZE, bitsize_args::shift_vis_out_one_module, enum_fills_bitsize, is_at_attribute,
    is_fallback_attribute, place_struct_fields, unreachable,
};

/// Intermediate Representation, just for bundling these together
struct ItemIr {
    /// generated item (and size check)
    expanded: TokenStream,
    ident: Ident,
    vis: Visibility,
}

pub(super) fn bitsize(args: TokenStream, item: TokenStream) -> manyhow::Result {
    let (item, mut args) = parse(item, args)?;
    forbid_item_at(&item)?;
    let attrs = SplitAttributes::from_item(&item)?;
    let ir = match item {
        Item::Struct(mut item) => {
            args.resolve_new_vis();
            let original_vis = item.vis.clone();
            if args.hide_value {
                // Re-exported from a private module; the inner type must be `pub`.
                item.vis = parse_quote!(pub);
                shift_field_vis_out_one_module(&mut item.fields);
            }
            modify_special_field_names(&mut item.fields);
            analyze_struct(&item.fields)?;
            let expanded = generate_struct(&item, args.bitsize)?;
            ItemIr {
                expanded,
                ident: item.ident,
                vis: original_vis,
            }
        }
        Item::Enum(item) => {
            if args.hide_value {
                bail!("`hide_value` is only supported on structs"; help = "enums do not have a `value` field")
            }
            if !args.is_default_new_vis() {
                bail!("`new` is only supported on structs"; help = "enums do not generate a `new` constructor")
            }
            analyze_enum(args.bitsize, item.variants.iter())?;
            let expanded = generate_enum(&item);
            ItemIr {
                expanded,
                ident: item.ident,
                vis: item.vis,
            }
        }
        _ => unreachable(()),
    };
    Ok(generate_common(ir, attrs, &args))
}

fn parse(item: TokenStream, args: TokenStream) -> manyhow::Result<(Item, BitsizeArgs)> {
    let item = syn::parse2(item).unwrap_or_else(unreachable);

    if args.is_empty() {
        bail!("missing attribute value"; help = "you need to define the size like this: `#[bitsize(32)]`")
    }

    let args = shared::parse_bitsize_args(args)?;
    Ok((item, args))
}

fn check_type_is_supported(ty: &Type) -> manyhow::Result<()> {
    use Type::*;
    match ty {
        Tuple(tuple) => tuple.elems.iter().try_for_each(check_type_is_supported)?,
        Array(array) => check_type_is_supported(&array.elem)?,
        // Probably okay (compilation would validate that this type is also Bitsized)
        Path(_) => (),
        // These don't work with structs or aren't useful in bitfields.
        BareFn(_) | Group(_) | ImplTrait(_) | Infer(_) | Macro(_) | Never(_) |
        // We could provide some info on error as to why Ptr/Reference won't work due to safety.
        Ptr(_) | Reference(_) |
        // The bitsize must be known at compile time.
        Slice(_) |
        // Something to investigate, but doesn't seem useful/usable here either.
        TraitObject(_) |
        // I have no idea where this is used.
        Verbatim(_) | Paren(_) => bail!(ty, "This field type is not supported"),
        _ => bail!(ty, "This field type is currently not supported"),
    }
    Ok(())
}

/// `hide_value` puts the struct one module deeper. Relative field vis is
/// shifted so it still means what the user wrote in their module.
fn shift_field_vis_out_one_module(fields: &mut Fields) {
    for field in fields.iter_mut() {
        field.vis = shift_vis_out_one_module(field.vis.clone());
    }
}

/// Allows you to give multiple fields the name `reserved` or `padding`
/// by numbering them for you.
fn modify_special_field_names(fields: &mut Fields) {
    // We could have just counted up, i.e. `reserved_0`, but people might interpret this as "reserved to zero".
    // Using some other, more useful unique info as postfix would be nice.
    // Also, it might be useful to generate no getters or setters for these fields and skipping some calc.
    let mut reserved_count = 0;
    let mut padding_count = 0;
    let field_idents_mut = fields.iter_mut().filter_map(|field| field.ident.as_mut());
    for ident in field_idents_mut {
        if ident == "reserved" || ident == "_reserved" {
            reserved_count += 1;
            let span = ident.span();
            let name = format!("reserved_{}", "i".repeat(reserved_count));
            *ident = Ident::new(&name, span)
        } else if ident == "padding" || ident == "_padding" {
            padding_count += 1;
            let span = ident.span();
            let name = format!("padding_{}", "i".repeat(padding_count));
            *ident = Ident::new(&name, span)
        }
    }
}

fn forbid_item_at(item: &Item) -> manyhow::Result<()> {
    let attrs = match item {
        Item::Struct(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        _ => return Ok(()),
    };
    for attr in attrs {
        if is_at_attribute(attr) {
            bail!(attr, "`#[at]` is only supported on struct fields");
        }
    }
    Ok(())
}

fn analyze_struct(fields: &Fields) -> manyhow::Result<()> {
    if fields.is_empty() {
        bail!("structs without fields are not supported")
    }

    // don't move this. we validate all nested field types here as well
    // and later assume this was checked.
    for field in fields {
        check_type_is_supported(&field.ty)?
    }
    Ok(())
}

fn analyze_enum(bitsize: BitSize, variants: Iter<Variant>) -> manyhow::Result<()> {
    if bitsize > MAX_ENUM_BIT_SIZE {
        bail!("enum bitsize is limited to {}", MAX_ENUM_BIT_SIZE)
    }

    let variant_count = variants.clone().count();
    if variant_count == 0 {
        bail!("empty enums are not supported");
    }

    for variant in variants.clone() {
        for attr in &variant.attrs {
            if is_at_attribute(attr) {
                bail!(attr, "`#[at]` is only supported on struct fields");
            }
        }
    }

    let has_fallback = variants.flat_map(|variant| &variant.attrs).any(is_fallback_attribute);

    if !has_fallback {
        // this has a side-effect of validating the enum count
        let _ = enum_fills_bitsize(bitsize, variant_count)?;
    }
    Ok(())
}

fn generate_struct(item: &ItemStruct, declared_bitsize: u8) -> manyhow::Result<TokenStream> {
    let ItemStruct { vis, ident, fields, .. } = item;
    let declared_bitsize = declared_bitsize as usize;
    let layout = place_struct_fields(fields, declared_bitsize)?;

    let size_check = if layout.uses_at {
        let asserts = &layout.asserts;
        quote! {
            const _: () = {
                #asserts
            };
        }
    } else {
        let computed_bitsize = fields.iter().fold(quote!(0), |acc, next| {
            let field_size = shared::generate_type_bitsize(&next.ty);
            quote!(#acc + #field_size)
        });
        quote! {
            // constness: when we get const blocks evaluated at compile time, add a const computed_bitsize
            const _: () = ::core::assert!(
                (#computed_bitsize) == (#declared_bitsize),
                concat!("struct size and declared bit size differ: ",
                // stringify!(#computed_bitsize),
                " != ",
                stringify!(#declared_bitsize))
            );
        }
    };

    // we could remove this if the whole struct gets passed
    let is_tuple_struct = fields.iter().any(|field| field.ident.is_none());
    let fields_def = if is_tuple_struct {
        let fields = fields.iter();
        quote! {
            ( #(#fields,)* );
        }
    } else {
        let fields = fields.iter();
        quote! {
            { #(#fields,)* }
        }
    };

    Ok(quote! {
        #vis struct #ident #fields_def

        #size_check
    })
}

// attributes are handled in `generate_common`
fn generate_enum(item: &ItemEnum) -> TokenStream {
    let ItemEnum { vis, ident, variants, .. } = item;
    quote! {
        #vis enum #ident {
            #variants
        }
    }
}

/// we have _one_ generate_common function, which holds everything that struct and enum have _in common_.
/// Everything else has its own generate_ functions.
fn generate_common(ir: ItemIr, attrs: SplitAttributes, args: &BitsizeArgs) -> TokenStream {
    let ItemIr { expanded, ident, vis } = ir;
    let SplitAttributes {
        before_compression,
        after_compression,
    } = attrs;

    let bitsize = args.bitsize;
    let extra = shared::internal_attr_options(args);
    let bitsize_internal_attr = quote! {#[::bilge::bitsize_internal(#bitsize #extra)]};

    let item = quote! {
        #(#before_compression)*
        #bitsize_internal_attr
        #(#after_compression)*
        #expanded
    };

    if args.hide_value {
        let mod_name = format_ident!("__bilge_{}", ident);
        quote! {
            #[doc(hidden)]
            #[allow(non_snake_case, unused_imports)]
            mod #mod_name {
                use super::*;
                #item
            }
            #vis use #mod_name::#ident;
        }
    } else {
        item
    }
}
