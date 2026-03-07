mod hart_local;

use {proc_macro::TokenStream, syn::Item};

#[proc_macro_attribute]
pub fn hart_local(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new_spanned(
            proc_macro2::TokenStream::from(attr),
            "hart_local does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let item = syn::parse_macro_input!(item as Item);
    let item = match item {
        Item::Static(item) => item,
        other_item => {
            return syn::Error::new_spanned(
                other_item,
                "hart_local can only be applied to static items",
            )
            .into_compile_error()
            .into();
        }
    };

    let output = hart_local::expand(item);

    TokenStream::from(output)
}
