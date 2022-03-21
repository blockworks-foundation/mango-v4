use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Pod)]
pub fn pod(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match data {
        syn::Data::Struct(_) => {
            quote! {
                unsafe impl bytemuck::Zeroable for #ident {}
                unsafe impl bytemuck::Pod for #ident {}
            }
        }

        _ => panic!(),
    }
    .into()
}
