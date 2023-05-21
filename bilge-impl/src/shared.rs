use proc_macro2::{TokenStream, Ident};
use proc_macro_error::{abort_call_site, emit_call_site_warning, abort};
use quote::{ToTokens, quote};
use syn::{DeriveInput, LitInt, Expr, punctuated::Iter, Variant, Type, Lit, ExprLit, Meta, Data, Attribute};

/// As arbitrary_int is limited to basic rust primitives, the maximum is u128.
/// Is there a true usecase for bitfields above this size?
/// This would also be change-worthy when rust starts supporting LLVM's arbitrary integers.
pub const MAX_STRUCT_BIT_SIZE: u8 = 128;
pub type BitSize = u8;

pub(crate) fn parse_derive(item: TokenStream) -> DeriveInput {
    syn::parse2(item).unwrap_or_else(unreachable)
}

// allow since we want `if try_from` blocks to stand out
#[allow(clippy::collapsible_if)]
pub(crate) fn analyze_derive(derive_input: &DeriveInput, try_from: bool) -> (&syn::Data, TokenStream, &Ident, BitSize, DeriveImpl) {
    let DeriveInput { 
        attrs,
        ident,
        // generics,
        data,
        ..
    } = derive_input;

    if !try_from {
        if attrs.iter().any(is_non_exhaustive_attribute) {
            abort_call_site!("Item can't be FromBits and non_exhaustive"; help = "remove #[non_exhaustive] or derive(FromBits) here")
        }
    } else {
        // currently not allowed, would need some thinking:
        if let syn::Data::Struct(_) = data {
            if attrs.iter().any(is_non_exhaustive_attribute) {
                abort_call_site!("Using #[non_exhaustive] on structs is currently not supported"; help = "open an issue on our repository if needed")
            }
        }
    }

    let derive_impl = match fallback_variant(data) {
        None if try_from => DeriveImpl::TryFrom,
        Some(_) if try_from => {
            emit_call_site_warning!(
                "enum defines fallback variant"; 
                help = "use `#[derive(FromBits)]` instead. a `From` implementation can be genereated from the fallback variant"
            );
            DeriveImpl::TryFrom
        }
        Some(variant) => DeriveImpl::FromWithFallbackVariant(variant),
        None => DeriveImpl::From,
    };

    // parsing the #[bitsize_internal(num)] attribute macro
    let args = attrs.iter().find_map(|attr| {
        if attr.to_token_stream().to_string().contains("bitsize_internal") {
            if let Meta::List(list) = &attr.meta {
                Some(list.tokens.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).unwrap_or_else(|| abort_call_site!("add #[bitsize] attribute above your derive attribute"));
    let (bitsize, arb_int) = bitsize_and_arbitrary_int_from(args);

    (data, arb_int, ident, bitsize, derive_impl)
}

// If we want to support bitsize(u4) besides bitsize(4), do that here.
pub fn bitsize_and_arbitrary_int_from(bitsize_arg: TokenStream) -> (BitSize, TokenStream) {
    let bitsize: LitInt = syn::parse2(bitsize_arg.clone()).unwrap_or_else(|_|
        abort!(bitsize_arg, "attribute value is not a number"; help = "you need to define the size like this: `#[bitsize(32)]`")
    );
    // without postfix
    let bitsize = bitsize.base10_parse().unwrap_or_else(|_|
        abort!(bitsize_arg, "attribute value is not a valid number"; help = "currently, numbers from 1 to {} are allowed", MAX_STRUCT_BIT_SIZE)
    );
    let arb_int = syn::parse_str(&format!("u{bitsize}")).unwrap_or_else(unreachable);
    (bitsize, arb_int)
}

pub fn generate_type_bitsize(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            tuple.elems.iter().map(generate_type_bitsize)
                .reduce(|acc, next| quote!((#acc + #next)))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        },
        Array(array) => {
            let elem_bitsize = generate_type_bitsize(&array.elem);
            let len_expr = &array.len;
            quote!((#elem_bitsize * #len_expr))
        },
        Path(_) => {
            quote!(<#ty as Bitsized>::BITS)
        },
        _ => unreachable(()),
    }
}

pub(crate) fn analyze_enum_derive(variants: Iter<Variant>, name: &Ident, internal_bitsize: BitSize, derive_impl: &DeriveImpl) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let variants_count = variants.len();
    // in enums, internal_bitsize <= 64; u64::MAX + 1 = u128
    let max_variants_count = 1u128 << internal_bitsize;
    
    // Verifying that the value doesn't exceed max_variants_count is done further down.
    let enum_fills_bitsize = variants_count as u128 == max_variants_count;
    validate_bitsize(derive_impl, enum_fills_bitsize);  

    let mut next_variant_value = 0;
    variants.map(|variant| {
        let variant_name = &variant.ident;
        let variant_value: u128 = match variant.discriminant.as_ref() {
            Some(d) => {
                let discriminant_expr = &d.1;
                match discriminant_expr {
                    Expr::Lit(ExprLit { lit: Lit::Int(int), .. }) => int.base10_parse().unwrap_or_else(unreachable),
                    _ => abort!(
                        discriminant_expr, "variant `{}` is not a number", variant_name;
                        help = "only literal integers currently supported"
                    )
                }
            }
            None => next_variant_value,
        };
        next_variant_value = variant_value + 1;

        if variant_value >= max_variants_count {
            abort_call_site!("Value {} exceeds the given number of bits", variant_name);
        }

        // might be useful for not generating "1u128 -> Self::Variant"
        let variant_value: Expr = syn::parse_str(&variant_value.to_string()).unwrap_or_else(unreachable);

        let from_int_match_arm = if matches!(derive_impl, DeriveImpl::TryFrom) {
            quote! {
                #variant_value => Ok(Self::#variant_name),
            }
        } else {
            quote! {
                #variant_value => Self::#variant_name,
            }
        };

        let to_int_match_arm = quote! {
            #name::#variant_name => Self::new(#variant_value),
        };

        (from_int_match_arm, to_int_match_arm)
    }).unzip()
}

/// Verify if the enum fills its bitsize, depending on which derive impl we are in.
fn validate_bitsize(derive_impl: &DeriveImpl, enum_fills_bitsize: bool) {
    match derive_impl {
        DeriveImpl::TryFrom if enum_fills_bitsize => {
            emit_call_site_warning!("enum fills its bitsize"; help = "you can use `#[derive(FromBits)]` instead, rust will provide `TryFrom` for you (so you don't necessarily have to update call-sites)");
        },
        DeriveImpl::FromWithFallbackVariant(_) if enum_fills_bitsize => {
            emit_call_site_warning!("enum fills its bitsize but has fallback variant"; help = "you can remove the #[fallback] attribute`");
        },
        DeriveImpl::From if !enum_fills_bitsize => {
            // semantically the same as #[non_exhaustive]
            abort_call_site!("enum doesn't fill its bitsize"; help = "you need to use `#[derive(TryFromBits)]` instead, or specify one of the variants as #[fallback]")
        },
        _ => (),
    }
}

pub(crate) fn generate_enum(arb_int: TokenStream, enum_type: &Ident, match_arms: (Vec<TokenStream>, Vec<TokenStream>), derive_impl: &DeriveImpl) -> TokenStream {
    let (from_int_match_arms, to_int_match_arms) = match_arms;

    let const_ = if cfg!(feature = "nightly") {
        quote!(const)
    } else {
        quote!()
    };

    let from_enum_impl = generate_from_enum_impl(&arb_int, enum_type, to_int_match_arms, &const_);
    let to_enum_impl = generate_to_enum_impl(&arb_int, enum_type, from_int_match_arms, &const_, derive_impl);

    quote! {
        #from_enum_impl
        #to_enum_impl
    }
}

pub(crate) fn generate_from_enum_impl(arb_int: &TokenStream, enum_type: &Ident, to_int_match_arms: Vec<TokenStream>, const_: &TokenStream) -> TokenStream {
    quote! {
        impl #const_ ::core::convert::From<#enum_type> for #arb_int {
            fn from(enum_value: #enum_type) -> Self {
                match enum_value {
                    #( #to_int_match_arms )*
                }
            }
        }
    }
}

fn generate_to_enum_impl(arb_int: &TokenStream, enum_type: &Ident, from_int_match_arms: Vec<TokenStream>, const_: &TokenStream, derive_impl: &DeriveImpl) -> TokenStream {
    match derive_impl {
        DeriveImpl::From => {
            quote! {
                impl #const_ ::core::convert::From<#arb_int> for #enum_type {
                    fn from(number: #arb_int) -> Self {
                        match number.value() {
                            #( #from_int_match_arms )*
                            // constness: unreachable!() is not const yet
                            _ => panic!("unreachable: arbitrary_int already validates that this is unreachable")
                        }
                    }
                }
            } 
        },
        DeriveImpl::FromWithFallbackVariant(fallback) => {
            let fallback_name = &fallback.ident;
            quote! {
                impl #const_ ::core::convert::From<#arb_int> for #enum_type {
                    fn from(number: #arb_int) -> Self {
                        match number.value() {
                            #( #from_int_match_arms )*
                            _ => Self::#fallback_name
                        }
                    }
                }
            }
        },
        DeriveImpl::TryFrom => {
            quote! {
                impl #const_ ::core::convert::TryFrom<#arb_int> for #enum_type {
                    type Error = #arb_int;
    
                    fn try_from(number: #arb_int) -> ::core::result::Result<Self, Self::Error> {
                        match number.value() {
                            #( #from_int_match_arms )*
                            i => Err(#arb_int::new(i)),
                        }
                    }
                }
            }
        },
    }
}

/// Filters fields which are always `FILLED`, meaning all bit-patterns are possible,
/// meaning they are (should be) From<uN>, not TryFrom<uN>
/// 
//TODO: We should maybe just rewrite this into something useful or add FILLED into Bitsized impls.
//otherwise, we could check if there is _not_ a struct or enum here by lower/uppercase first letter
pub fn is_always_filled(ty: &Type) -> bool {
    let ty = ty.to_token_stream().to_string();
    ty.starts_with('u') || ty == "bool"
}


#[inline]
pub fn unreachable<T, U>(_: T) -> U {
    unreachable!("should have already been validated")
}

fn fallback_variant(data: &Data) -> Option<Variant> {
    match data {
        Data::Enum(enum_data) => {
            let mut variants_with_fallback = enum_data
                .variants
                .iter()
                .filter(|variant| variant.attrs.iter().any(is_fallback_attribute));

            let variant = variants_with_fallback.next();

            if variants_with_fallback.next().is_some() {
                abort_call_site!("only one enum variant may be fallback"; help = "remove #[fallback] attributes until you only have one");
            } else {
                variant.cloned()
            }
        }
        Data::Struct(struct_data) => {
            let mut field_attrs = struct_data.fields.iter().flat_map(|field| &field.attrs);
            
            if field_attrs.any(is_fallback_attribute) {
                abort_call_site!("the attribute `fallback` is only applicable to enums"; help = "remove all `#[fallback]` from this struct")
            } else {
                None
            }
        }
        _ => unreachable(())
    }
}

pub(crate) enum DeriveImpl {
    From,
    FromWithFallbackVariant(Variant),
    TryFrom,
}

impl DeriveImpl {
    pub fn into_fallback_variant(self) -> Option<Variant> {
        match self {
            DeriveImpl::FromWithFallbackVariant(fallback) => Some(fallback),
            _ => None,
        }
    }
}

pub fn is_attribute(attr: &Attribute, name: &str) -> bool {
    if let Meta::Path(path) = &attr.meta {
        path.is_ident(name)
    } else {
        false
    }
}

fn is_non_exhaustive_attribute(attr: &Attribute) -> bool {
    is_attribute(attr, "non_exhaustive")
}

fn is_fallback_attribute(attr: &Attribute) -> bool {
    is_attribute(attr, "fallback")
}