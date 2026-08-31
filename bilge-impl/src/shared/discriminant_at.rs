use manyhow::{bail, ensure};
use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Fields, LitInt, Token, Type, Variant};

/// `#[discriminant_at(n)]` or `#[discriminant_at(n..=m)]` on a bitsized enum.
/// Tag bits must sit at the LSB (`0..=k`) or MSB (`k..=N-1`); the rest is payload.
pub struct DiscriminantAt {
    pub start: usize,
    pub width: usize,
    pub span: Span,
}

impl DiscriminantAt {
    pub fn end(&self) -> usize {
        self.start + self.width - 1
    }

    pub fn payload_width(&self, bitsize: usize) -> usize {
        bitsize - self.width
    }

    pub fn tag_at_lsb(&self) -> bool {
        self.start == 0
    }

    pub fn validate(&self, bitsize: usize) -> manyhow::Result<()> {
        if self.width == 0 || self.end() >= bitsize {
            bail!(
                self.span,
                "discriminant occupies bits {}..={} but the enum is only {bitsize} bits",
                self.start,
                self.end()
            );
        }
        let at_lsb = self.start == 0;
        let at_msb = self.end() == bitsize - 1;
        if !at_lsb && !at_msb {
            bail!(
                self.span,
                "discriminant in the middle is not supported";
                help = "place it at the LSB (`#[discriminant_at(0..=k)]`) or the MSB (`#[discriminant_at(k..={})]`)",
                bitsize - 1
            );
        }
        Ok(())
    }

    /// Expressions in terms of a `raw` integer (the backing value).
    pub fn extract_tag(&self, bitsize: usize) -> TokenStream {
        let start = self.start;
        if self.width == bitsize {
            quote!(raw)
        } else if self.tag_at_lsb() {
            let mask = Literal::u128_unsuffixed(self.mask());
            quote!(raw & #mask)
        } else {
            quote!(raw >> #start)
        }
    }

    pub fn extract_payload(&self, bitsize: usize) -> TokenStream {
        if self.width == bitsize {
            quote!(0)
        } else if self.tag_at_lsb() {
            let width = self.width;
            quote!(raw >> #width)
        } else {
            let mask = Literal::u128_unsuffixed(self.payload_mask(bitsize));
            quote!(raw & #mask)
        }
    }

    fn mask(&self) -> u128 {
        (1u128 << self.width) - 1
    }

    fn payload_mask(&self, bitsize: usize) -> u128 {
        (1u128 << self.payload_width(bitsize)) - 1
    }

    /// Combine a `tag` and `payload` integer into the backing value.
    pub fn combine(&self, bitsize: usize) -> TokenStream {
        let width = self.width;
        let start = self.start;
        if width == bitsize {
            quote!(tag)
        } else if self.tag_at_lsb() {
            quote!((payload << #width) | tag)
        } else {
            quote!((tag << #start) | payload)
        }
    }
}

impl Parse for DiscriminantAt {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Ident) {
            return Err(input.error("use `#[discriminant(Type)]` for a tag that lives in a different value, not `#[discriminant_at]`"));
        }

        let start_lit: LitInt = input.parse()?;
        let start: usize = start_lit.base10_parse()?;
        let start_span = start_lit.span();

        if input.is_empty() {
            return Ok(DiscriminantAt {
                start,
                width: 1,
                span: start_span,
            });
        }

        if input.peek(Token![..=]) {
            input.parse::<Token![..=]>()?;
            let end_lit: LitInt = input.parse()?;
            let end: usize = end_lit.base10_parse()?;
            let span = start_span.join(end_lit.span()).unwrap_or(start_span);
            if end < start {
                return Err(syn::Error::new(span, "range start is greater than end"));
            }
            return Ok(DiscriminantAt {
                start,
                width: end - start + 1,
                span,
            });
        }

        if input.peek(Token![..]) {
            return Err(input.error("use an inclusive range: `#[discriminant_at(n..=m)]`"));
        }

        Err(input.error("expected `#[discriminant_at(n)]` or `#[discriminant_at(n..=m)]`"))
    }
}

pub fn is_discriminant_at_attribute(attr: &Attribute) -> bool {
    attr.path().is_ident("discriminant_at")
}

pub fn parse_discriminant_at(attrs: &[Attribute]) -> manyhow::Result<Option<DiscriminantAt>> {
    let mut found = None;
    for attr in attrs {
        if !is_discriminant_at_attribute(attr) {
            continue;
        }
        ensure!(found.is_none(), attr, "duplicate `#[discriminant_at]`");
        found = Some(attr.parse_args()?);
    }
    Ok(found)
}

/// `Some(ty)` for a one-field tuple variant, `None` for a unit variant.
pub fn variant_payload_ty(variant: &Variant) -> manyhow::Result<Option<&Type>> {
    match &variant.fields {
        Fields::Unit => Ok(None),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(Some(&fields.unnamed[0].ty)),
        Fields::Unnamed(_) => {
            bail!(variant, "payload variant must have exactly one field"; help = "put the rest of the bits in one bitsized type")
        }
        Fields::Named(_) => {
            bail!(variant, "payload variants must be tuple variants"; help = "use `Variant(Payload)` instead of named fields")
        }
    }
}

pub fn payload_from_int_arm(
    disc: &DiscriminantAt, bitsize: usize, variant: &syn::Ident, payload_ty: Option<&Type>, tag: &proc_macro2::Literal, try_from: bool,
) -> TokenStream {
    match payload_ty {
        None => {
            if try_from {
                quote! { #tag => Ok(Self::#variant), }
            } else {
                quote! { #tag => Self::#variant, }
            }
        }
        Some(ty) => {
            let payload_raw = disc.extract_payload(bitsize);
            if try_from {
                let payload_start = if disc.tag_at_lsb() { disc.width } else { 0 };
                quote! {
                    #tag => {
                        let payload_raw = #payload_raw;
                        let payload_bits = <#ty as Bitsized>::ArbitraryInt::new(
                            payload_raw as <<#ty as Bitsized>::ArbitraryInt as Integer>::UnderlyingType
                        );
                        match <#ty>::try_from(payload_bits) {
                            Ok(payload) => Ok(Self::#variant(payload)),
                            Err(e) => Err(::bilge::IntoBitsError::into_bits_error(
                                e,
                                stringify!(#ty),
                                payload_bits.value() as u128,
                                <#ty as Bitsized>::BITS as u8,
                            )
                            .at_offset(#payload_start)),
                        }
                    }
                }
            } else {
                quote! {
                    #tag => {
                        let payload_raw = #payload_raw;
                        let payload_bits = <#ty as Bitsized>::ArbitraryInt::new(
                            payload_raw as <<#ty as Bitsized>::ArbitraryInt as Integer>::UnderlyingType
                        );
                        match <#ty>::try_from(payload_bits) {
                            Ok(payload) => Self::#variant(payload),
                            Err(_) => ::core::panic!("unreachable"),
                        }
                    }
                }
            }
        }
    }
}

pub fn payload_to_int_arm(
    disc: &DiscriminantAt, bitsize: usize, enum_name: &syn::Ident, variant: &syn::Ident, payload_ty: Option<&Type>, tag: &proc_macro2::Literal,
    arb_int: &TokenStream, by_value: bool,
) -> TokenStream {
    let combine = disc.combine(bitsize);
    match payload_ty {
        None => quote! {
            #enum_name::#variant => {
                let tag = #tag as <#arb_int as Integer>::UnderlyingType;
                let payload = 0 as <#arb_int as Integer>::UnderlyingType;
                #arb_int::new(#combine)
            }
        },
        Some(ty) => {
            let payload_int = if by_value {
                quote! { <#ty as Bitsized>::ArbitraryInt::from(payload) }
            } else {
                quote! {{
                    use ::bilge::Bitsized as _;
                    payload.as_int()
                }}
            };
            quote! {
                #enum_name::#variant(payload) => {
                    let tag = #tag as <#arb_int as Integer>::UnderlyingType;
                    let payload = #payload_int.value() as <#arb_int as Integer>::UnderlyingType;
                    #arb_int::new(#combine)
                }
            }
        }
    }
}
