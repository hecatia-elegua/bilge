mod split;

use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, abort_call_site};
use quote::{quote, quote_spanned};
use split::SplitAttributes;
use syn::{punctuated::Iter, spanned::Spanned, Fields, Item, ItemEnum, ItemStruct, Type, Variant};

use crate::shared::{self, enum_fills_bitsize, is_fallback_attribute, unreachable, BitSize, FieldLayout, MAX_ENUM_BIT_SIZE};

/// Intermediate Representation, just for bundling these together
struct ItemIr {
    /// generated item (and size check)
    expanded: TokenStream,
}

pub(super) fn bitsize(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, declared_bitsize, layout) = parse(item, args);
    let attrs = SplitAttributes::from_item(&item);
    let ir = match item {
        Item::Struct(mut item) => {
            modify_special_field_names(&mut item.fields);
            analyze_struct(&item.fields);
            let expanded = generate_struct(&item, declared_bitsize, layout);
            ItemIr { expanded }
        }
        Item::Enum(item) => {
            analyze_enum(declared_bitsize, item.variants.iter());
            let expanded = generate_enum(&item);
            ItemIr { expanded }
        }
        _ => unreachable(()),
    };
    generate_common(ir, attrs, declared_bitsize, layout)
}

fn parse(item: TokenStream, args: TokenStream) -> (Item, BitSize, FieldLayout) {
    let item = syn::parse2(item).unwrap_or_else(unreachable);

    if args.is_empty() {
        abort_call_site!("missing attribute arguments"; help = "need arguments like this: `#[bitsize(32)]` or `#[bitsize(32, manual)]")
    }

    let (declared_bitsize, _arb_int, layout) = shared::bitsize_and_arbitrary_int_from(args);
    (item, declared_bitsize, layout)
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

fn generate_struct(item: &ItemStruct, declared_bitsize: u8, layout: FieldLayout) -> TokenStream {
    let ItemStruct { vis, ident, fields, .. } = item;

    let declared_bitsize = declared_bitsize as usize;
    let fields_check = match layout {
        FieldLayout::Auto => generate_auto_layout_check(fields, declared_bitsize),
        FieldLayout::Manual => generate_manual_layout_check(fields, declared_bitsize),
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

    quote! {
        #vis struct #ident #fields_def

        #fields_check
    }
}

fn generate_auto_layout_check(fields: &Fields, declared_bitsize: usize) -> TokenStream {
    fields.iter().for_each(|f| {
        if let Some(attr) = shared::find_bit_range_attr(&f.attrs) {
            abort!(attr.raw().span(), "#[bit]/#[bits] is forbidden under auto layout")
        }
    });
    let computed_bitsize = fields.iter().fold(quote!(0), |acc, next| {
        let field_size = shared::generate_type_bitsize(&next.ty);
        quote!(#acc + #field_size)
    });
    quote!(
        // constness: when we get const blocks evaluated at compile time, add a const computed_bitsize
        const _: () = assert!(
            (#computed_bitsize) == (#declared_bitsize),
            concat!("struct size and declared bit size differ: ",
            // stringify!(#computed_bitsize),
            " != ",
            stringify!(#declared_bitsize))
        );
    )
}

fn generate_manual_layout_check(fields: &Fields, declared_bitsize: usize) -> TokenStream {
    fields
        .iter()
        .map(|f| {
            let Some(attr) = shared::find_bit_range_attr(&f.attrs) else {
                abort!(f.span(), "required #[bit]/#[bits] attribute under manual layout")
            };
            let range = attr.parse().unwrap_or_else(|e| abort!(e.span(), e));
            if range.start_bit + range.bit_size > declared_bitsize {
                abort!(attr.raw().span(), "#[bit]/#[bits] exceeds struct size");
            }
            let declared_bits = range.bit_size;
            let actual_bits = shared::generate_type_bitsize(&f.ty);
            let msg = format!("declared size ({declared_bits}) not match actual field size");
            // generate code to check field's bit size
            quote_spanned!(attr.raw().span()=>
                const _: () = {
                    if #actual_bits != #declared_bits {
                        panic!(#msg);
                    }
                };
            )
        })
        .collect()
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
fn generate_common(ir: ItemIr, attrs: SplitAttributes, declared_bitsize: u8, layout: FieldLayout) -> TokenStream {
    let ItemIr { expanded } = ir;
    let SplitAttributes {
        before_compression,
        after_compression,
    } = attrs;

    // before_compression.iter_mut().for_each(|attr| if attr.path() =="bit" );

    let layout = layout.ident();
    let bitsize_internal_attr = quote! {#[::bilge::bitsize_internal(#declared_bitsize, #layout)]};

    quote! {
        #(#before_compression)*
        #bitsize_internal_attr
        #(#after_compression)*
        #expanded
    }
}
