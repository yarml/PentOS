use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStatic;

pub(crate) fn expand(item: ItemStatic) -> TokenStream {
    let output = quote! {
        #item
    };

    output
}