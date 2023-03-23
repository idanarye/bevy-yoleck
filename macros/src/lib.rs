use proc_macro2::TokenStream;

use quote::quote;
use syn::{DeriveInput, Error};

#[proc_macro_derive(YoleckComponent)]
pub fn derive_yoleck_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match impl_yoleck_component_derive(input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_yoleck_component_derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = input.ident;
    let key = name.to_string();
    let result = quote!(
        impl YoleckComponent for #name {
            const KEY: &'static str = #key;
        }
    );
    Ok(result)
}
