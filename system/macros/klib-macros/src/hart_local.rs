use {
    proc_macro2::{Ident, Span, TokenStream},
    quote::quote,
    syn::{Error, Item, ItemStatic, Result, Visibility},
};

pub(crate) fn handle_common(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
    hart_crate: TokenStream,
) -> proc_macro::TokenStream {
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

    match expand(item, hart_crate) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand(item: ItemStatic, hart_crate: TokenStream) -> Result<TokenStream> {
    let ItemStatic {
        attrs,
        vis,
        mutability,
        ident,
        ty,
        expr,
        ..
    } = item;

    if let syn::StaticMutability::Mut(mut_token) = mutability {
        return Err(Error::new_spanned(
            mut_token,
            "`mut` is not needed for #[hart_local] statics; \
             HartLocal provides interior mutability per-hart",
        ));
    }

    let backing_ident = Ident::new(&format!("__HART_LOCAL_{}", ident), Span::call_site());

    let vis = match &vis {
        Visibility::Public(_) => quote! { pub },
        Visibility::Restricted(r) => quote! { #r },
        Visibility::Inherited => quote! {},
    };

    let extra_attrs = &attrs;

    let expanded = quote! {
        #[unsafe(link_section = ".hart_local")]
        #[used]
        // Make it mut just so that accessing it is unsafe
        static mut #backing_ident: #hart_crate::Wrapper<#ty> = #hart_crate::Wrapper::new(#expr);

        #(#extra_attrs)*
        #vis static #ident: #hart_crate::HartLocal<#ty> = unsafe { #hart_crate::HartLocal::new(&#backing_ident) };
    };

    Ok(expanded)
}
