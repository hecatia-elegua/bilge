use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(CustomBits)]
pub fn custom_bits_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_custom_bits_macro(&ast)
}

fn impl_custom_bits_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields_number = match &ast.data {
        syn::Data::Struct(d) => d.fields.len(),
        syn::Data::Enum(d) => d.variants.len(),
        syn::Data::Union(d) => d.fields.named.len(),
    };
    let gen = quote! {
        impl CustomBits for #name {
            fn fields(&self) -> usize{
                #fields_number
            }
        }
    };
    gen.into()
}
