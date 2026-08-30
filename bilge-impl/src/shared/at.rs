use manyhow::{bail, ensure};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, Field, Fields, Lit, LitInt, Token, Type};

use super::{generate_type_bitsize, last_ident_of_path};

/// `#[at(n)]` or `#[at(n..=m)]` on a struct field.
pub struct AtSpec {
    pub start: usize,
    /// Inclusive end, when the user wrote `n..=m`.
    pub end: Option<usize>,
    pub span: Span,
}

impl AtSpec {
    pub fn range_width(&self) -> Option<usize> {
        self.end.map(|end| end - self.start + 1)
    }
}

impl Parse for AtSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start_lit: LitInt = input.parse()?;
        let start: usize = start_lit.base10_parse()?;
        let start_span = start_lit.span();

        if input.is_empty() {
            return Ok(AtSpec {
                start,
                end: None,
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
            return Ok(AtSpec { start, end: Some(end), span });
        }

        if input.peek(Token![..]) {
            return Err(input.error("use an inclusive range: `#[at(n..=m)]`"));
        }

        Err(input.error("expected `#[at(n)]` or `#[at(n..=m)]`"))
    }
}

pub fn is_at_attribute(attr: &Attribute) -> bool {
    attr.path().is_ident("at")
}

pub fn parse_field_at(field: &Field) -> manyhow::Result<Option<AtSpec>> {
    let mut found = None;
    for attr in &field.attrs {
        if !is_at_attribute(attr) {
            continue;
        }
        ensure!(found.is_none(), attr, "duplicate `#[at]`");
        found = Some(attr.parse_args()?);
    }
    Ok(found)
}

/// Attributes copied onto getters/setters must not include `#[at]` or `#[default]`.
pub fn attrs_without_at(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attr| !is_at_attribute(attr) && !super::is_default_attribute(attr))
        .collect()
}

pub struct FieldPlacement {
    pub offset: TokenStream,
    pub width: TokenStream,
    pub offset_n: Option<usize>,
    pub width_n: Option<usize>,
}

pub struct StructLayout {
    pub fields: Vec<FieldPlacement>,
    pub uses_at: bool,
    /// Extra `const` checks when `#[at]` is used (holes are allowed, so we no
    /// longer require `sum(field bits) == declared`).
    pub asserts: TokenStream,
}

/// Sequential fields without `#[at]` still fill from bit 0 upward.
/// `#[at(n)]` jumps the cursor to bit `n` (0 = LSB). Holes are implicit
/// reserved bits. Overlap or going backwards is an error.
pub fn place_struct_fields(fields: &Fields, declared_bitsize: usize) -> manyhow::Result<StructLayout> {
    let mut cursor_n: Option<usize> = Some(0);
    let mut cursor_ts = quote!(0);
    let mut uses_at = false;
    let mut placements = Vec::with_capacity(fields.len());
    let mut asserts = TokenStream::new();

    for (i, field) in fields.iter().enumerate() {
        let at = parse_field_at(field)?;
        if at.is_some() {
            uses_at = true;
        }

        let width_ts = generate_type_bitsize(&field.ty);
        let width_n = try_type_bitsize(&field.ty);
        let desc = field_desc(field, i);

        let (offset_n, offset_ts, at_span) = match &at {
            Some(spec) => {
                if spec.start >= declared_bitsize {
                    bail!(
                        spec.span,
                        "field {desc} starts at bit {} but the struct is only {declared_bitsize} bits",
                        spec.start
                    );
                }
                if let Some(end) = spec.end {
                    if end >= declared_bitsize {
                        bail!(spec.span, "field {desc} ends at bit {end} but the struct is only {declared_bitsize} bits");
                    }
                }
                if let Some(cursor) = cursor_n {
                    if spec.start < cursor {
                        bail!(
                            spec.span,
                            "fields overlap or are reordered";
                            help = "field {desc} starts at bit {}, but the next unused bit is {cursor}",
                            spec.start
                        );
                    }
                } else {
                    let start = spec.start;
                    asserts.extend(quote! {
                        ::core::assert!(
                            (#start) >= (#cursor_ts),
                            "fields overlap or are reordered"
                        );
                    });
                }

                if let Some(range_width) = spec.range_width() {
                    if let Some(width) = width_n {
                        if width != range_width {
                            bail!(
                                spec.span,
                                "`#[at({}..={})]` is {range_width} bits, but field {desc} is {width} bits",
                                spec.start,
                                spec.end.unwrap()
                            );
                        }
                    } else {
                        asserts.extend(quote! {
                            ::core::assert!(
                                (#width_ts) == (#range_width),
                                "`#[at(n..=m)]` does not match the field type's bitsize"
                            );
                        });
                    }
                }

                let start = spec.start;
                (Some(start), quote!(#start), Some(spec.span))
            }
            None => (cursor_n, cursor_ts.clone(), None),
        };

        if let (Some(start), Some(width)) = (offset_n, width_n) {
            if start + width > declared_bitsize {
                let span = at_span.unwrap_or_else(|| field.span());
                bail!(
                    span,
                    "field {desc} extends past the declared bitsize of {declared_bitsize}";
                    help = "it occupies bits {start}..{}",
                    start + width
                );
            }
        } else {
            asserts.extend(quote! {
                ::core::assert!(
                    (#offset_ts) + (#width_ts) <= (#declared_bitsize),
                    "field extends past the declared bitsize"
                );
            });
        }

        let end_ts = quote!((#offset_ts) + (#width_ts));
        cursor_n = match (offset_n, width_n) {
            (Some(start), Some(width)) => Some(start + width),
            _ => None,
        };
        cursor_ts = end_ts;

        placements.push(FieldPlacement {
            offset: offset_ts,
            width: width_ts,
            offset_n,
            width_n,
        });
    }

    Ok(StructLayout {
        fields: placements,
        uses_at,
        asserts,
    })
}

/// Covering ranges from bit 0 to `declared_bitsize`, including holes, LSB first.
/// Used by `BinaryBits` so implicit padding still appears in the bit dump.
pub fn binary_segments(layout: &StructLayout, declared_bitsize: usize) -> Vec<(TokenStream, TokenStream)> {
    let mut segments = Vec::new();
    let mut pos_n = Some(0usize);
    let mut pos_ts = quote!(0);

    let push_hole =
        |segments: &mut Vec<_>, pos_n: Option<usize>, pos_ts: &TokenStream, next_n: Option<usize>, next_ts: &TokenStream| match (pos_n, next_n) {
            (Some(from), Some(to)) if to > from => {
                let width = to - from;
                segments.push((quote!(#from), quote!(#width)));
            }
            (Some(from), Some(to)) if to == from => {}
            _ => {
                segments.push((pos_ts.clone(), quote!((#next_ts) - (#pos_ts))));
            }
        };

    for field in &layout.fields {
        push_hole(&mut segments, pos_n, &pos_ts, field.offset_n, &field.offset);
        segments.push((field.offset.clone(), field.width.clone()));
        pos_n = match (field.offset_n, field.width_n) {
            (Some(start), Some(width)) => Some(start + width),
            _ => None,
        };
        let offset = &field.offset;
        let width = &field.width;
        pos_ts = quote!((#offset) + (#width));
    }

    let declared_ts = quote!(#declared_bitsize);
    push_hole(&mut segments, pos_n, &pos_ts, Some(declared_bitsize), &declared_ts);

    segments
}

pub fn try_type_bitsize(ty: &Type) -> Option<usize> {
    match ty {
        Type::Tuple(tuple) => {
            let mut sum = 0usize;
            for elem in &tuple.elems {
                sum += try_type_bitsize(elem)?;
            }
            Some(sum)
        }
        Type::Array(array) => {
            let len: usize = match &array.len {
                Expr::Lit(ExprLit { lit: Lit::Int(n), .. }) => n.base10_parse().ok()?,
                _ => return None,
            };
            Some(try_type_bitsize(&array.elem)? * len)
        }
        Type::Path(_) => last_ident_of_path(ty).and_then(super::bitsize_from_type_ident).map(|n| n as usize),
        _ => None,
    }
}

fn field_desc(field: &Field, index: usize) -> String {
    match &field.ident {
        Some(name) => format!("`{name}`"),
        None => format!("{index}"),
    }
}
