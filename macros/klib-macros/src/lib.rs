mod hart_local;

use {proc_macro::TokenStream, quote::quote};

#[proc_macro_attribute]
pub fn hart_local(attr: TokenStream, item: TokenStream) -> TokenStream {
    hart_local::handle_common(attr, item, quote! { klib::hart })
}

#[proc_macro_attribute]
pub fn klib_hart_local(attr: TokenStream, item: TokenStream) -> TokenStream {
    hart_local::handle_common(attr, item, quote! { crate::hart })
}
