use {
    builder::{Config, Target},
    quote::{format_ident, quote},
};

fn main() {
    builder::configure(Config {
        target: Target::Elf64,
    });
    let assemblies = [];
    builder::add_nasm_lib("pent-kernel-asm", &assemblies);

    let force_links = chef_core::crates::all_drivers().map(|c| {
        let driver = c.driver.clone().unwrap();
        let id = driver.id;

        let c_name = format_ident!("{}", c.name);
        let fn_name = format_ident!("__force_link_driver_{id}");

        quote! {
            #c_name::#fn_name();
        }
    });

    let force_link = quote! {
        fn __force_link_drivers() {
            #(#force_links)*
        }
    };

    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(
        format!("{out_dir}/force_link_drivers.rs"),
        force_link.to_string(),
    )
    .unwrap();
}
