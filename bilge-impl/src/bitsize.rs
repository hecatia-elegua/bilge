use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, abort};
use quote::{quote, ToTokens};
use syn::{Item, ItemStruct, ItemEnum, Attribute, Fields, Meta, parse_quote, spanned::Spanned};
use crate::shared::{self, BitSize, unreachable};

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
    /// generated item (and size check)
    expanded: TokenStream,
}

pub(super) fn bitsize(args: TokenStream, item: TokenStream) -> TokenStream {
    let (item, declared_bitsize) = parse(item, args);
    let attrs = split_attributes(&item);
    let ir = match item {
        Item::Struct(mut item) => {
            modify_special_field_names(&mut item.fields);
            let expanded = generate_struct(&item, declared_bitsize);
            ItemIr { expanded }
        }
        Item::Enum(item) => {
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

/// Split item attributes into those applied before bitfield-compression and those applied after.
/// Also, abort on any invalid configuration.
/// 
/// Any derives with suffix `Bits` will be able to access field information.
/// This way, users of `bilge` can define their own derives working on the uncompressed bitfield.
fn split_attributes(item: &Item) -> SplitAttributes {
    match item {
        //enums don't need special handling
        Item::Enum(item) => SplitAttributes { 
            before_compression: item.attrs.clone(),
            after_compression: vec![]
        },
        Item::Struct(item) => {
            let mut from_bytes = None;
            let mut has_frombits = false;
            let mut before_compression = vec![];
            let mut after_compression = vec![];
            for attr in &item.attrs {
                if attr.to_token_stream().to_string().contains("bitsize_internal") {
                    abort!(attr, "remove bitsize_internal"; help = "attribute bitsize_internal can only be applied internally by the bitsize macros")
                }
                match &attr.meta {
                    Meta::List(list) => {
                        if !list.path.is_ident("derive") {
                            // It is most probable that basic attr macros work if we put them on after compression
                            after_compression.push(attr.clone());
                            continue;
                        }
                        attr.parse_nested_meta(|meta| {
                            let derive_path = meta.path;
                            if derive_path.is_ident("Debug") {
                                abort!(derive_path, "use DebugBits for structs")
                            }
                            let derive = parse_quote!(#[derive(#derive_path)]);

                            let derive_path_str = derive_path.get_ident().unwrap_or_else(|| {
                                // We could just use the last path segment or use `derive_str.contains()` but that sounds breakable.
                                // Handling this for real might be easy, I just don't know how right now.
                                abort!(derive_path, "we currently only support simple derives, without paths.");
                            }).to_string();
                            match derive_path_str.as_str() {
                                "FromBytes" => {
                                    from_bytes = Some(derive_path);
                                    after_compression.push(derive);
                                }
                                "FromBits" => {
                                    has_frombits = true;
                                    before_compression.push(derive)
                                }
                                path => {
                                    if path.ends_with("Bits") {
                                        before_compression.push(derive);
                                    } else {
                                        // It is most probable that basic derive macros work if we put them on after compression
                                        after_compression.push(derive);
                                    }
                                }
                            }
                            Ok(())
                        }).unwrap_or_else(unreachable)
                    }
                    // I don't know with which attrs I can hit Path and NameValue,
                    // so let's just put them on after compression.
                    _ => after_compression.push(attr.clone()),
                }
            }
            if let Some(from_bytes) = from_bytes {
                if !has_frombits {
                    abort!(from_bytes, "a bitfield struct with zerocopy::FromBytes also needs to have FromBits")
                }
            }
            SplitAttributes { before_compression, after_compression }
        },
        _ => abort_call_site!("item is not a struct or enum"; help = "`#[bitsize]` can only be used on structs and enums")
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