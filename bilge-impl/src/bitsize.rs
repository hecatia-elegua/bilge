use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, abort};
use quote::quote;
use syn::{punctuated::Iter, Variant, Item, ItemStruct, ItemEnum, Type, Attribute, Fields, Meta, parse_quote, spanned::Spanned, Path};
use crate::shared::{self, BitSize, unreachable, enum_fills_bitsize, is_fallback_attribute};

/// As `#[repr(u128)]` is unstable and currently no real usecase for higher sizes exists, the maximum is u64.
const MAX_ENUM_BIT_SIZE: BitSize = 64;

/// Since we want to be maximally interoperable, we need to handle attributes in a special way.
/// We use `#[bitsize]` as a sort of scope for all attributes below it and
/// the whole family of `-Bits` macros only works when used in that scope.
/// 
/// Let's visualize why this is the case, starting with some user-code:
/// ```ignore
/// #[bitsize(6)]
/// #[derive(Clone, Copy, PartialEq, DebugBits, FromBits)]
/// struct Example {
///     field1: u2,
///     field2: u4,
/// }
/// ```
/// First, the attributes get sorted, depending on their name.
/// Every attribute in need of field information gets resolved first,
/// in this case `DebugBits` and `FromBits`.
/// 
/// Now, after resolving all `before_compression` attributes, the halfway-resolved
/// code looks like this:
/// ```ignore
/// #[bilge::bitsize_internal(6)]
/// #[derive(Clone, Copy, PartialEq)]
/// struct Example {
///     field1: u2,
///     field2: u4,
/// }
/// ```
/// This `#[bitsize_internal]` attribute is the one actually doing the compression and generating
/// all the getters, setters and a constructor.
/// 
/// Finally, the struct ends up like this (excluding the generated impl blocks):
/// ```ignore
/// struct Example {
///     value: u6,
/// }
/// ```
struct SplitAttributes {
    before_compression: Vec<Attribute>,
    after_compression: Vec<Attribute>,
}

/// Intermediate Representation, just for bundling these together
struct ItemIr {
    name: Ident,
    /// needed in from_bits and try_from_bits
    filled_check: TokenStream,
    /// generated item (and size check)
    expanded: TokenStream,
}

pub(super) fn bitsize(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, declared_bitsize) = parse(item, args);
    let (attrs, needs_default_safe_check) = split_attributes(&item);
    let ir = match item {
        Item::Struct(mut item) => {
            modify_special_field_names(&mut item.fields);
            let name = item.ident.clone();
            let filled_check = analyze_struct(&item.fields);
            let expanded = generate_struct(&item, declared_bitsize, needs_default_safe_check);
            ItemIr { name, filled_check, expanded }
        }
        Item::Enum(item) => {
            let name = item.ident.clone();
            let (filled_check, is_default_safe_check) = analyze_enum(declared_bitsize, item.variants.iter());
            let expanded = generate_enum(&item, is_default_safe_check);
            ItemIr { name, filled_check, expanded }
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

/// a crude approximation of the things we currently consider in item attributes
enum ParsedAttribute<'a> {
    Derives(Vec<Path>),
    BitsizeInternal(&'a Meta),
    SomethingElse(&'a Meta),
}

impl<'a> ParsedAttribute<'a> {
    fn parse(attr: &'a Attribute) -> ParsedAttribute<'a> {
        match &attr.meta {
            Meta::List(list) if list.path.is_ident("derive") => {
                let mut derives = Vec::new();
                list.parse_nested_meta(|meta| {
                    derives.push(meta.path);
                    Ok(())
                }).unwrap_or_else(unreachable);
                ParsedAttribute::Derives(derives)
            },
            Meta::List(list) if list.path.is_ident("bitsize_internal") => ParsedAttribute::BitsizeInternal(&attr.meta),
            meta => ParsedAttribute::SomethingElse(meta),
        }
    }
}

/// Split item attributes into those applied before bitfield-compression and those applied after.
/// Also, abort on any invalid configuration.
/// 
/// Any derives with suffix `Bits` will be able to access field information.
/// This way, users of `bilge` can define their own derives working on the uncompressed bitfield.
fn split_attributes(item: &Item) -> (SplitAttributes, bool) {
    match item {
        //enums don't need special handling
        Item::Enum(item) => {
            let split = SplitAttributes { 
                before_compression: item.attrs.clone(),
                after_compression: vec![]
            };
            let needs_derive_default_check = false;
            (split, needs_derive_default_check)
        }
        Item::Struct(item) => {
            let mut from_bytes = None;
            let mut derives_frombits = false;
            let mut derives_default = false;
            let mut before_compression = vec![];
            let mut after_compression = vec![];
            for attr in &item.attrs {
                match ParsedAttribute::parse(attr) {
                    ParsedAttribute::BitsizeInternal(_) => abort!(
                        attr, 
                        "remove bitsize_internal"; 
                        help = "attribute bitsize_internal can only be applied internally by the bitsize macros"
                    ),
                    ParsedAttribute::Derives(derives) => {
                        for derive_path in derives {
                            let Some(derive_path_str) = derive_path.get_ident() else {
                                // We could just use the last path segment or use `derive_str.contains()` but that sounds breakable.
                                // Handling this for real might be easy, I just don't know how right now.
                                abort!(derive_path, "we currently only support simple derives, without paths.");
                            };

                            let derive = parse_quote!(#[derive(#derive_path)]);

                            match derive_path_str.to_string().as_str() {
                                "Debug" => abort!(derive_path, "use DebugBits for structs"),
                                "FromBytes" => {
                                    from_bytes = Some(derive_path);
                                    after_compression.push(derive);
                                }
                                "FromBits" => {
                                    derives_frombits = true;
                                    before_compression.push(derive)
                                }
                                "Default" => {
                                    // as of right now, we consider this to be safe in combination with emitted is_default_safe
                                    derives_default = true;
                                    after_compression.push(derive);
                                }
                                path if path.ends_with("Bits") => before_compression.push(derive),
                                _ => {
                                    // It is most probable that basic derive macros work if we put them on after compression
                                    after_compression.push(derive);
                                }
                            }
                        }
                        
                    },

                    // If this is a `Meta::List` with some ident as its path, but is not a derive,
                    // then it is most probable that basic attr macros work if we put them on after compression.
                    // Otherwise it's some attribute we haven't considered yet and currently assume to be harmless. 
                    // I don't know with which attrs I can hit Path and NameValue,
                    // so let's just put them on after compression.
                    ParsedAttribute::SomethingElse(_) => after_compression.push(attr.clone()),
                }
            }
                
            if let Some(from_bytes) = from_bytes {
                if !derives_frombits {
                    abort!(from_bytes, "a bitfield struct with zerocopy::FromBytes also needs to have FromBits")
                }
            }
            let split = SplitAttributes { before_compression, after_compression };
            (split, derives_default)
        },
        _ => abort_call_site!("item is not a struct or enum"; help = "`#[bitsize]` can only be used on structs and enums")
    }
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
            quote!(#ty::FILLED)
        },
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

fn analyze_struct(fields: &Fields) -> TokenStream {
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

fn analyze_enum(bitsize: BitSize, variants: Iter<Variant>) -> (TokenStream, TokenStream) {
    let variant_count = variants.clone().count();
    if variant_count == 0 {
        abort_call_site!("empty enums are not supported");
    }

    if bitsize > MAX_ENUM_BIT_SIZE {
        abort_call_site!("enum bitsize is limited to {}", MAX_ENUM_BIT_SIZE)
    }

    // an enum is `#[derive(Default)]` safe if any of its variants can have a discriminant of 0
    let is_default_safe_check = generate_can_enum_be_zero(variants.clone()); 
        
    let has_fallback = variants.flat_map(|variant| &variant.attrs).any(is_fallback_attribute);

    let filled_check = if has_fallback {
        quote!(true)
    } else {
        let enum_is_filled = enum_fills_bitsize(bitsize, variant_count);
        quote!(#enum_is_filled)
    };

    (filled_check, is_default_safe_check)
}


/// for variants without an explicit discriminant, rustc assigns a discriminant
/// which is 1 higher than variant below it, and the default assignment is 0.
/// however we already reject negative discriminants at some point.
/// so an enum can only have 0 as a discriminant if the first variant has no explicit discriminant,
/// or if any of the variants explicitly assign to 0.
fn generate_can_enum_be_zero(variants: Iter<Variant>) -> TokenStream {
    let first_variant = variants.clone().next().unwrap_or_else(|| unreachable(())); // we reject empty enums
    
    if first_variant.discriminant.is_none() {
        quote!(true)
    } else {
        variants
            .filter_map(|variant| {
                if let Some(disc) = variant.discriminant.as_ref() {
                    let disc_expr = &disc.1;
                    Some(quote!(#disc_expr == 0))
                } else {
                    None
                }
            })
            .reduce(|acc, discriminant_equals_zero| quote! { (#acc || #discriminant_equals_zero) })
            .unwrap_or_else(|| unreachable(()))
    }
}

fn generate_default_safety_check(fields: &Fields, struct_name: &Ident) -> TokenStream {
    fn safety_check(ty: &Type) -> TokenStream {
        match ty {
            Type::Path(_) => quote!(#ty::DEFAULT_SAFE),
            Type::Array(inner) => safety_check(&inner.elem),
            Type::Tuple(inner) => inner
                .elems
                .iter()
                .map(safety_check)
                .reduce(|acc, next| quote!((#acc && #next)))
                .unwrap_or_else(|| quote!(true)),
            _ => unreachable(()),
        }
    }

    let struct_check = fields
        .iter()
        .map(|field| safety_check(&field.ty))
        .reduce(|acc, next| quote!((#acc && #next)))
        .unwrap_or_else(|| unreachable(()));

    quote! {
        const _: () = assert!(
            #struct_check,
            concat!(
                "struct ",
                stringify!(#struct_name),
                " cannot safely derive Default because one of its fields is an enum that cannot have a value of 0"
            )
        );
    }
}

fn generate_struct(item: &ItemStruct, declared_bitsize: u8, needs_default_safe_check: bool) -> TokenStream {
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

    let default_safety_check = if needs_default_safe_check {
        generate_default_safety_check(fields, ident)
    } else {
        quote!()
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
        
        #default_safety_check
    }
}

// attributes are handled in `generate_common`
fn generate_enum(item: &ItemEnum, is_default_safe_check: TokenStream) -> TokenStream {
    let ItemEnum { vis, ident, variants, .. } = item;
    quote! {
        #vis enum #ident {
            #variants
        }

        impl #ident {
            const DEFAULT_SAFE: bool = #is_default_safe_check;
        }
    }
}

/// we have _one_ generate_common function, which holds everything that struct and enum have _in common_.
/// Everything else has its own generate_ functions.
fn generate_common(ir: ItemIr, attrs: SplitAttributes, declared_bitsize: u8) -> TokenStream {
    let ItemIr { name: item_type, filled_check, expanded } = ir;
    let SplitAttributes { before_compression, after_compression } = attrs;

    let bitsize_internal_attr =  quote! {#[bilge::bitsize_internal(#declared_bitsize)]};

    quote! {
        #(#before_compression)*
        #bitsize_internal_attr
        #(#after_compression)*
        #expanded

        impl #item_type {
            pub const FILLED: bool = #filled_check;
        }
    }
}