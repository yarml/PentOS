use {
    proc_macro::TokenStream,
    quote::{format_ident, quote},
    syn::ItemFn,
};

pub(crate) fn handle(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new_spanned(
            proc_macro2::TokenStream::from(attr),
            "driver does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let crt = chef_core::crates::find_crate(&std::env::var("CARGO_PKG_NAME").unwrap());

    let Some(driver) = crt.driver.as_ref() else {
        return syn::Error::new_spanned(
            proc_macro2::TokenStream::from(item),
            "missing or invalid driver.toml",
        )
        .into_compile_error()
        .into();
    };

    let drv_name = &driver.name;
    let drv_id = &driver.id;
    let drv_desc = &driver.description;

    let func = syn::parse_macro_input!(item as ItemFn);
    let fn_name = func.sig.ident.clone();

    let force_link_fn_name = format_ident!("__force_link_driver_{drv_id}");

    quote! {
        #func

        #[used]
        static __DRIVER: klib::dev::Driver = klib::dev::Driver {
            init: #fn_name,
            id: #drv_id,
            name: #drv_name,
            description: #drv_desc,
        };

        #[used]
        #[unsafe(link_section = ".driver")]
        static __DRIVER_PTR: &klib::dev::Driver = &__DRIVER;

        pub fn #force_link_fn_name() {}
    }
    .into()
}
