extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(ViewCore)]
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
        impl ViewCore for #name {
            fn base_view(&self) -> &BaseView {
                &self.view
            }
            fn base_view_mut(&mut self) -> &mut BaseView {
                &mut self.view
            }
        }
    };
    gen.into()
}
