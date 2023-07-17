mod split;

use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, abort};
use quote::quote;
use syn::{punctuated::Iter, Variant, Item, ItemStruct, ItemEnum, Type, Fields, spanned::Spanned};
use crate::shared::{self, BitSize, unreachable, enum_fills_bitsize, is_fallback_attribute, MAX_ENUM_BIT_SIZE};
use split::{split_item_attributes, SplitAttributes};

/// Intermediate Representation, just for bundling these together
struct ItemIr {
    /// generated item (and size check)
    expanded: TokenStream,
}

pub(super) fn bitsize(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, declared_bitsize) = parse(item, args);
    let attrs = split_item_attributes(&item);
    let ir = match item {
        Item::Struct(mut item) => {
            modify_special_field_names(&mut item.fields);
            analyze_struct(&item.fields);
            let expanded = generate_struct(&item, declared_bitsize);
            ItemIr { expanded }
        }
        Item::Enum(item) => {
            analyze_enum(declared_bitsize, item.variants.iter());
            let expanded = generate_enum(&item);
            ItemIr { expanded }
        }
        _ => unreachable(()),
    };
    generate_common(ir, attrs, declared_bitsize)
}

fn parse(item: TokenStream, args: TokenStream) -> (Item, BitSize) {
    let item = syn::parse2(item).unwrap_or_else(unreachable);

    if args.is_empty() {
        abort_call_site!("missing attribute value"; help = "you need to define the size like this: `#[bitsize(32)]`")
    }
    
    let (declared_bitsize, _arb_int) = shared::bitsize_and_arbitrary_int_from(args);
    (item, declared_bitsize)
}

fn check_type_is_supported(ty: &Type) {
    use Type::*;
    match ty {
        Tuple(tuple) => tuple.elems.iter().for_each(check_type_is_supported),
        Array(array) => check_type_is_supported(&array.elem),
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
        Verbatim(_) | Paren(_) => abort!(ty, "This field type is not supported"),
        _ => abort!(ty, "This field type is currently not supported"),
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

fn analyze_struct(fields: &Fields) {
    if fields.is_empty() {
        abort_call_site!("structs without fields are not supported")
    }

    // don't move this. we validate all nested field types here as well
    // and later assume this was checked.
    for field in fields {
        check_type_is_supported(&field.ty)
    }
}

fn analyze_enum(bitsize: BitSize, variants: Iter<Variant>) {
    if bitsize > MAX_ENUM_BIT_SIZE {
        abort_call_site!("enum bitsize is limited to {}", MAX_ENUM_BIT_SIZE)
    }

    let variant_count = variants.clone().count();
    if variant_count == 0 {
        abort_call_site!("empty enums are not supported");
    }

    let has_fallback = variants.flat_map(|variant| &variant.attrs).any(is_fallback_attribute);

    if !has_fallback {
        // this has a side-effect of validating the enum count
        let _ = enum_fills_bitsize(bitsize, variant_count);
    }
}

fn generate_struct(item: &ItemStruct, declared_bitsize: u8) -> TokenStream {
    let ItemStruct { vis, ident, fields, .. } = item;
    let declared_bitsize = declared_bitsize as usize;

    let computed_bitsize = fields.iter().fold(quote!(0), |acc, next| {
        let field_size = shared::generate_type_bitsize(&next.ty);
        quote!(#acc + #field_size)
    });

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

    quote! {
        #vis struct #ident #fields_def

        // constness: when we get const blocks evaluated at compile time, add a const computed_bitsize
        const _: () = assert!(
            (#computed_bitsize) == (#declared_bitsize),
            concat!("struct size and declared bit size differ: ",
            // stringify!(#computed_bitsize),
            " != ",
            stringify!(#declared_bitsize))
        );
    }
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
fn generate_common(ir: ItemIr, attrs: SplitAttributes, declared_bitsize: u8) -> TokenStream {
    let ItemIr { expanded } = ir;
    let SplitAttributes { before_compression, after_compression } = attrs;

    let bitsize_internal_attr =  quote! {#[bilge::bitsize_internal(#declared_bitsize)]};

    quote! {
        #(#before_compression)*
        #bitsize_internal_attr
        #(#after_compression)*
        #expanded
    }
}