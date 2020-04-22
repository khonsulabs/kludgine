extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(View)]
pub fn derive_view(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_view(&ast)
}

fn impl_view(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        #[async_trait]
        impl ViewCore for #name {
            async fn style(&self) -> Style {
                self.view.style
            }
        }
    };
    gen.into()
}
