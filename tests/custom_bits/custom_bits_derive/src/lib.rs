use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(FieldsInBits)]
pub fn fields_in_bits(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    generate(&ast)
}

fn generate(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let count = match &ast.data {
        syn::Data::Struct(d) => d.fields.len(),
        syn::Data::Enum(d) => d.variants.len(),
        syn::Data::Union(d) => d.fields.named.len(),
    };
    let gen = quote! {
        impl FieldsInBits for #name {
            fn field_count() -> usize {
                #count
            }
        }
    };
    gen.into()
}
