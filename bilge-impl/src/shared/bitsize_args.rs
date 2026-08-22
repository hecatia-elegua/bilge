use manyhow::{bail, ensure};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::{Ident, LitInt, PathSegment, Token, VisRestricted, Visibility, parse_quote};

use super::{BitSize, MAX_STRUCT_BIT_SIZE, unreachable};

/// Parsed `#[bitsize(...)]` / `#[bitsize_internal(...)]` arguments.
pub struct BitsizeArgs {
    pub bitsize: BitSize,
    pub arb_int: TokenStream,
    pub hide_value: bool,
    pub new_vis: Visibility,
    /// Whether the user wrote `new = ...`.
    pub new_specified: bool,
}

impl BitsizeArgs {
    pub fn is_default_new_vis(&self) -> bool {
        !self.new_specified
    }

    /// Interpret `new` vis as written in the user's module. `hide_value` nests
    /// one extra module, so relative vis gets one additional `super`.
    pub fn resolve_new_vis(&mut self) {
        if self.hide_value {
            self.new_vis = shift_vis_out_one_module(self.new_vis.clone());
        }
    }
}

pub fn parse_bitsize_args(args: TokenStream) -> manyhow::Result<BitsizeArgs> {
    ensure!(
        !args.is_empty(),
        "missing attribute value"; help = "you need to define the size like this: `#[bitsize(32)]`"
    );

    let (lit, rest) = match Parser::parse2(parse_lit_and_rest, args.clone()) {
        Ok(parsed) => parsed,
        Err(_) => {
            bail!(args, "attribute value is not a number"; help = "you need to define the size like this: `#[bitsize(32)]`")
        }
    };

    let number_span = if rest.is_empty() { args } else { lit.to_token_stream() };
    ensure!(
        let Some(bitsize) = lit.base10_parse().ok().filter(|&n| n != 0 && n <= MAX_STRUCT_BIT_SIZE),
        number_span, "attribute value is not a valid number"; help = "currently, numbers from 1 to {} are allowed", MAX_STRUCT_BIT_SIZE
    );
    let arb_int = syn::parse_str(&format!("u{bitsize}")).unwrap_or_else(unreachable);

    let (hide_value, new_vis, new_specified) = parse_options(rest)?;
    Ok(BitsizeArgs {
        bitsize,
        arb_int,
        hide_value,
        new_vis,
        new_specified,
    })
}

fn parse_lit_and_rest(input: ParseStream) -> syn::Result<(LitInt, TokenStream)> {
    let lit: LitInt = input.parse()?;
    let rest: TokenStream = input.parse()?;
    Ok((lit, rest))
}

fn parse_options(rest: TokenStream) -> manyhow::Result<(bool, Visibility, bool)> {
    if rest.is_empty() {
        return Ok((false, Visibility::Inherited, false));
    }
    let options: BitsizeOptions = syn::parse2(rest)?;
    Ok((options.hide_value, options.new_vis, options.new_specified))
}

struct BitsizeOptions {
    hide_value: bool,
    new_vis: Visibility,
    new_specified: bool,
}

impl Parse for BitsizeOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut hide_value = false;
        let mut new_vis: Option<Visibility> = None;

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }

            let ident: Ident = input.parse()?;
            if ident == "hide_value" {
                if hide_value {
                    return Err(syn::Error::new(ident.span(), "duplicate `hide_value`"));
                }
                hide_value = true;
            } else if ident == "new" {
                if new_vis.is_some() {
                    return Err(syn::Error::new(ident.span(), "duplicate `new`"));
                }
                input.parse::<Token![=]>()?;
                new_vis = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unknown bitsize option `{ident}`; expected `hide_value` or `new = <vis>`"),
                ));
            }
        }

        Ok(BitsizeOptions {
            hide_value,
            new_specified: new_vis.is_some(),
            new_vis: new_vis.unwrap_or(Visibility::Inherited),
        })
    }
}

/// Shift visibility one module outward. Relative vis (`pub(super)`, `pub(self)`,
/// inherited) is interpreted as written in the user's module; `pub` / `pub(crate)`
/// / `pub(in crate::…)` stay as-is.
pub fn shift_vis_out_one_module(vis: Visibility) -> Visibility {
    match vis {
        Visibility::Inherited => parse_quote!(pub(super)),
        Visibility::Public(_) => vis,
        Visibility::Restricted(restricted) => shift_restricted_vis(restricted),
    }
}

fn shift_restricted_vis(mut restricted: VisRestricted) -> Visibility {
    let Some(first_ident) = restricted.path.segments.first().map(|s| s.ident.clone()) else {
        return Visibility::Restricted(restricted);
    };
    match first_ident.to_string().as_str() {
        "crate" => Visibility::Restricted(restricted),
        "self" => {
            restricted.path.segments[0].ident = Ident::new("super", first_ident.span());
            if restricted.path.segments.len() == 1 {
                restricted.in_token = None;
            } else {
                restricted.in_token = Some(Default::default());
            }
            Visibility::Restricted(restricted)
        }
        "super" => {
            restricted.in_token = Some(Default::default());
            restricted
                .path
                .segments
                .insert(0, PathSegment::from(Ident::new("super", first_ident.span())));
            Visibility::Restricted(restricted)
        }
        _ => Visibility::Restricted(restricted),
    }
}

/// Tokens to append inside `#[bitsize_internal(N ...)]`.
pub fn internal_attr_options(args: &BitsizeArgs) -> TokenStream {
    let mut opts = TokenStream::new();
    if args.hide_value {
        opts.extend(quote!(, hide_value));
    }
    // `bitsize_internal` defaults to a private `new`; only emit when it differs.
    if !matches!(args.new_vis, Visibility::Inherited) {
        let vis = &args.new_vis;
        opts.extend(quote!(, new = #vis));
    }
    opts
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    fn shift(input: &str) -> String {
        let vis = if input.is_empty() {
            Visibility::Inherited
        } else {
            syn::parse_str(input).unwrap()
        };
        shift_vis_out_one_module(vis).to_token_stream().to_string()
    }

    fn vis(s: &str) -> String {
        let parsed: Visibility = syn::parse_str(s).unwrap();
        parsed.to_token_stream().to_string()
    }

    #[test]
    fn relative_vis_gets_one_super() {
        assert_eq!(shift(""), vis("pub(super)"));
        assert_eq!(shift("pub(self)"), vis("pub(super)"));
        assert_eq!(shift("pub(super)"), vis("pub(in super::super)"));
        assert_eq!(shift("pub(in super)"), vis("pub(in super::super)"));
        assert_eq!(shift("pub(in super::super)"), vis("pub(in super::super::super)"));
        assert_eq!(shift("pub(in self::foo)"), vis("pub(in super::foo)"));
    }

    #[test]
    fn crate_and_pub_vis_are_unchanged() {
        assert_eq!(shift("pub"), vis("pub"));
        assert_eq!(shift("pub(crate)"), vis("pub(crate)"));
        assert_eq!(shift("pub(in crate::foo)"), vis("pub(in crate::foo)"));
    }
}
