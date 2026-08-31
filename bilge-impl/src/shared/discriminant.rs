use manyhow::{bail, ensure};
use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Type};

use super::discriminant_at::DiscriminantAt;
use super::{MAX_ENUM_BIT_SIZE, bitsize_from_type_ident, last_ident_of_path, parse_discriminant_at};

/// `#[discriminant(TagType)]` on a bitsized enum: the tag is a separate value,
/// not bits in the payload integer.
pub struct Discriminant {
    pub ty: Type,
    pub span: Span,
}

impl Discriminant {
    /// Width when the tag type is `uN` / `bool`. Custom bitsized types are unknown here.
    pub fn known_width(&self) -> Option<u8> {
        last_ident_of_path(&self.ty).and_then(bitsize_from_type_ident)
    }

    pub fn assigner_width(&self) -> u8 {
        self.known_width().unwrap_or(MAX_ENUM_BIT_SIZE)
    }

    /// Rejects `uN` / `bool` wider than `MAX_ENUM_BIT_SIZE` at expansion.
    /// Custom bitsized types are checked later with a const assert on `BITS`.
    pub fn validate(&self) -> manyhow::Result<()> {
        if let Some(width) = self.known_width() {
            if width > MAX_ENUM_BIT_SIZE {
                bail!(
                    self.span,
                    "discriminant tag type is limited to {} bits",
                    MAX_ENUM_BIT_SIZE;
                    help = "use `u1`..=`u64`, `bool`, or a bitsized type of at most {} bits",
                    MAX_ENUM_BIT_SIZE
                );
            }
        }
        Ok(())
    }
}

impl Parse for Discriminant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("expected a single type, e.g. `#[discriminant(CrtcIndex)]`"));
        }
        let span = syn::spanned::Spanned::span(&ty);
        Ok(Discriminant { ty, span })
    }
}

pub fn is_discriminant_attribute(attr: &Attribute) -> bool {
    attr.path().is_ident("discriminant")
}

pub fn parse_discriminant(attrs: &[Attribute]) -> manyhow::Result<Option<Discriminant>> {
    let mut found = None;
    for attr in attrs {
        if !is_discriminant_attribute(attr) {
            continue;
        }
        ensure!(found.is_none(), attr, "duplicate `#[discriminant]`");
        found = Some(attr.parse_args().map_err(|_| {
            syn::Error::new_spanned(
                attr,
                "expected a tag type: `#[discriminant(CrtcIndex)]`\n\
                 help: use `#[discriminant_at(n..=m)]` to place the tag in this integer",
            )
        })?);
    }
    Ok(found)
}

/// In-band tag bits vs a tag that lives in a different value.
#[allow(clippy::large_enum_variant)]
pub enum EnumDiscriminant {
    At(DiscriminantAt),
    Type(Discriminant),
}

pub fn parse_enum_discriminant(attrs: &[Attribute]) -> manyhow::Result<Option<EnumDiscriminant>> {
    let at = parse_discriminant_at(attrs)?;
    let ty = parse_discriminant(attrs)?;
    match (at, ty) {
        (Some(_), Some(disc)) => {
            bail!(
                disc.span,
                "`#[discriminant]` and `#[discriminant_at]` cannot be used together";
                help = "`#[discriminant_at(n..=m)]` places the tag in this integer; `#[discriminant(Type)]` uses a separate tag value"
            )
        }
        (Some(at), None) => Ok(Some(EnumDiscriminant::At(at))),
        (None, Some(ty)) => {
            ty.validate()?;
            Ok(Some(EnumDiscriminant::Type(ty)))
        }
        (None, None) => Ok(None),
    }
}

pub fn from_pair_arm(variant: &syn::Ident, payload_ty: Option<&Type>, tag: &Literal, try_from: bool) -> TokenStream {
    match payload_ty {
        None => {
            if try_from {
                quote! { #tag => Ok(Self::#variant), }
            } else {
                quote! { #tag => Self::#variant, }
            }
        }
        Some(ty) => {
            if try_from {
                quote! {
                    #tag => match <#ty>::try_from(data) {
                        Ok(payload) => Ok(Self::#variant(payload)),
                        Err(e) => Err(::bilge::IntoBitsError::into_bits_error(
                            e,
                            stringify!(#ty),
                            data.value() as u128,
                            <#ty as Bitsized>::BITS as u8,
                        )),
                    },
                }
            } else {
                quote! { #tag => Self::#variant(<#ty>::from(data)), }
            }
        }
    }
}

pub fn to_pair_arm(
    enum_name: &syn::Ident, variant: &syn::Ident, payload_ty: Option<&Type>, tag: &Literal, tag_ty: &Type, arb_int: &TokenStream,
) -> TokenStream {
    let tag_value = quote! {
        {
            type TagInt = <#tag_ty as Bitsized>::ArbitraryInt;
            let tag_bits = <TagInt as Integer>::new(#tag as <TagInt as Integer>::UnderlyingType);
            match <#tag_ty>::try_from(tag_bits) {
                Ok(tag) => tag,
                Err(_) => ::core::panic!("unreachable"),
            }
        }
    };
    match payload_ty {
        None => quote! {
            #enum_name::#variant => {
                let tag = #tag_value;
                let data = #arb_int::new(0);
                (tag, data)
            }
        },
        Some(_) => quote! {
            #enum_name::#variant(payload) => {
                let tag = #tag_value;
                let data = #arb_int::from(payload);
                (tag, data)
            }
        },
    }
}

/// Payload bits only, from `&self` (or owned, via autoderef).
pub fn payload_only_to_int_arm(enum_name: &syn::Ident, variant: &syn::Ident, payload_ty: Option<&Type>, arb_int: &TokenStream) -> TokenStream {
    match payload_ty {
        None => quote! {
            #enum_name::#variant => #arb_int::new(0),
        },
        Some(_) => quote! {
            #enum_name::#variant(payload) => {
                use ::bilge::Bitsized as _;
                payload.as_int()
            },
        },
    }
}

pub fn generate_pair_from_enum(
    enum_type: &syn::Ident, tag_ty: &Type, arb_int: &TokenStream, to_pair_arms: &[TokenStream], const_: &TokenStream,
) -> TokenStream {
    quote! {
        impl #enum_type {
            /// Returns the tag and the payload bits.
            pub #const_ fn to_tag_and_data(self) -> (#tag_ty, #arb_int) {
                match self {
                    #( #to_pair_arms )*
                }
            }
        }
        impl #const_ ::core::convert::From<#enum_type> for (#tag_ty, #arb_int) {
            fn from(enum_value: #enum_type) -> Self {
                enum_value.to_tag_and_data()
            }
        }
    }
}

pub fn generate_try_from_pair(
    enum_type: &syn::Ident, tag_ty: &Type, arb_int: &TokenStream, from_pair_arms: &[TokenStream], const_: &TokenStream,
) -> TokenStream {
    quote! {
        impl #const_ ::core::convert::TryFrom<(#tag_ty, #arb_int)> for #enum_type {
            type Error = ::bilge::BitsError;

            fn try_from((tag, data): (#tag_ty, #arb_int)) -> ::core::result::Result<Self, Self::Error> {
                let tag_raw = <#tag_ty as Bitsized>::ArbitraryInt::from(tag).value();
                match tag_raw {
                    #( #from_pair_arms )*
                    _ => Err(::bilge::give_me_error(
                        stringify!(#enum_type),
                        tag_raw as u128,
                        <#tag_ty as Bitsized>::BITS as u8,
                    )),
                }
            }
        }
    }
}

pub fn generate_from_pair(
    enum_type: &syn::Ident, tag_ty: &Type, arb_int: &TokenStream, from_pair_arms: &[TokenStream], const_: &TokenStream, fill_check: TokenStream,
    assumes: &[TokenStream],
) -> TokenStream {
    quote! {
        #fill_check
        impl #const_ ::core::convert::From<(#tag_ty, #arb_int)> for #enum_type {
            fn from((tag, data): (#tag_ty, #arb_int)) -> Self {
                #( #assumes )*
                let tag_raw = <#tag_ty as Bitsized>::ArbitraryInt::from(tag).value();
                match tag_raw {
                    #( #from_pair_arms )*
                    _ => ::core::panic!("unreachable: arbitrary_int already validates that this is unreachable"),
                }
            }
        }
    }
}
