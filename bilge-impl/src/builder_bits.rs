use manyhow::{bail, ensure};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Expr, Field, Fields, Type, Visibility};

use crate::shared::{self, parse_field_default, unreachable};

struct BuilderField {
    name: Ident,
    ty: Type,
    default: Option<Expr>,
}

pub(crate) fn builder_bits(item: TokenStream) -> manyhow::Result {
    let derive_input = shared::parse_derive(item);
    let (new_vis, name, fields) = analyze(&derive_input)?;
    let builder_fields = collect_fields(fields)?;
    Ok(generate(name, &derive_input.vis, &new_vis, &builder_fields))
}

fn analyze(derive_input: &DeriveInput) -> manyhow::Result<(Visibility, &Ident, &Fields)> {
    ensure!(
        let Some(args) = derive_input.attrs.iter().find_map(shared::bitsize_internal_arg),
        "add #[bitsize] attribute above your derive attribute"
    );
    let args = shared::parse_bitsize_args(args)?;

    match &derive_input.data {
        Data::Struct(data) => Ok((args.new_vis, &derive_input.ident, &data.fields)),
        Data::Enum(_) => bail!("BuilderBits is only supported on structs"; help = "enums do not generate a constructor"),
        Data::Union(_) => unreachable(()),
    }
}
fn collect_fields(fields: &Fields) -> manyhow::Result<Vec<BuilderField>> {
    let mut out = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let name = field_name(field, i);
        let name_str = name.to_string();
        let default = parse_field_default(field)?;
        shared::reject_default_on_reserved(field, &name_str, default.is_some(), false)?;
        if shared::is_reserved_or_padding(&name_str) {
            continue;
        }
        out.push(BuilderField {
            name,
            ty: field.ty.clone(),
            default,
        });
    }
    Ok(out)
}

fn field_name(field: &Field, i: usize) -> Ident {
    field.ident.clone().unwrap_or_else(|| format_ident!("val_{i}"))
}

fn generate(struct_name: &Ident, struct_vis: &Visibility, new_vis: &Visibility, fields: &[BuilderField]) -> TokenStream {
    let builder_name = format_ident!("{struct_name}Builder");
    let type_params: Vec<Ident> = fields.iter().map(|f| format_ident!("__{}", f.name)).collect();

    let builder_struct_fields = type_params.iter().map(|tp| quote!(#tp: #tp));

    let unset_inits = type_params.iter().map(|tp| quote!(#tp: ()));

    let unset_types = fields.iter().map(|_| quote!(()));
    let builder_ty_unset = if type_params.is_empty() {
        quote!(#builder_name)
    } else {
        quote!(#builder_name<#(#unset_types),*>)
    };

    let setters = fields
        .iter()
        .enumerate()
        .map(|(i, field)| generate_setter(field, i, &builder_name, &type_params, fields, new_vis));

    let build_impl = generate_build(struct_name, &builder_name, new_vis, fields, &type_params);
    let finish_traits = generate_finish_traits(struct_name, fields);

    let type_param_decl = if type_params.is_empty() { quote!() } else { quote!(<#(#type_params),*>) };

    quote! {
        #[allow(non_camel_case_types, unused_qualifications)]
        #[must_use]
        #struct_vis struct #builder_name #type_param_decl {
            #(#builder_struct_fields,)*
        }

        impl #struct_name {
            /// Final method to call on the builder. Required fields must be set exactly once before `build`.
            #[must_use]
            #new_vis fn builder() -> #builder_ty_unset {
                #builder_name {
                    #(#unset_inits,)*
                }
            }
        }

        #finish_traits
        #(#setters)*
        #build_impl
    }
}

fn generate_setter(
    field: &BuilderField, index: usize, builder_name: &Ident, type_params: &[Ident], fields: &[BuilderField], vis: &Visibility,
) -> TokenStream {
    let name = &field.name;
    let ty = &field.ty;
    let impl_params: Vec<&Ident> = type_params.iter().enumerate().filter(|(j, _)| *j != index).map(|(_, id)| id).collect();

    let from_args = type_params
        .iter()
        .enumerate()
        .map(|(j, tp)| if j == index { quote!(()) } else { quote!(#tp) });
    let to_args = type_params
        .iter()
        .enumerate()
        .map(|(j, tp)| if j == index { quote!(#ty) } else { quote!(#tp) });

    let rebind = fields
        .iter()
        .zip(type_params.iter())
        .enumerate()
        .map(|(j, (_, tp))| if j == index { quote!(#tp: #name) } else { quote!(#tp: self.#tp) });

    let impl_generics = if impl_params.is_empty() { quote!() } else { quote!(<#(#impl_params),*>) };

    quote! {
        #[allow(non_camel_case_types)]
        impl #impl_generics #builder_name<#(#from_args),*> {
            #[must_use]
            #vis fn #name(self, #name: #ty) -> #builder_name<#(#to_args),*> {
                #builder_name {
                    #(#rebind,)*
                }
            }
        }
    }
}

fn generate_finish_traits(struct_name: &Ident, fields: &[BuilderField]) -> TokenStream {
    let mut out = TokenStream::new();
    for field in fields {
        let ty = &field.ty;
        let trait_name = finish_trait(struct_name, &field.name);
        let on_unimplemented = if field.default.is_none() {
            let name = field.name.to_string();
            let message = format!("missing required builder field `{name}`");
            let label = format!("call `.{name}(...)` before `.build()`");
            quote! {
                #[diagnostic::on_unimplemented(message = #message, label = #label)]
            }
        } else {
            quote!()
        };
        let unit_impl = if let Some(default) = &field.default {
            quote! {
                impl #trait_name for () {
                    #[inline]
                    fn finish(self) -> #ty {
                        #default
                    }
                }
            }
        } else {
            quote!()
        };
        out.extend(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #on_unimplemented
            trait #trait_name {
                fn finish(self) -> #ty;
            }
            impl #trait_name for #ty {
                #[inline]
                fn finish(self) -> #ty {
                    self
                }
            }
            #unit_impl
        });
    }
    out
}

fn finish_trait(struct_name: &Ident, field: &Ident) -> Ident {
    format_ident!("__{struct_name}_finish_{field}")
}

fn generate_build(struct_name: &Ident, builder_name: &Ident, vis: &Visibility, fields: &[BuilderField], type_params: &[Ident]) -> TokenStream {
    if type_params.is_empty() {
        return quote! {
            impl #builder_name {
                #vis fn build(self) -> #struct_name {
                    #struct_name::new()
                }
            }
        };
    }

    let bounds = fields.iter().zip(type_params).map(|(f, tp)| {
        let tr = finish_trait(struct_name, &f.name);
        quote!(#tp: #tr)
    });
    let new_args = type_params.iter().map(|tp| quote!(self.#tp.finish()));

    quote! {
        #[allow(non_camel_case_types)]
        impl<#(#type_params),*> #builder_name<#(#type_params),*> {
            #vis fn build(self) -> #struct_name
            where
                #(#bounds,)*
            {
                #struct_name::new(#(#new_args),*)
            }
        }
    }
}
