use {
    proc_macro2::{Ident, Span, TokenStream},
    quote::quote,
    syn::{Error, ItemStatic, Result, Visibility},
};

pub(crate) fn expand(item: ItemStatic) -> Result<TokenStream> {
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
        static #backing_ident: #ty = #expr;

        #(#extra_attrs)*
        #vis static #ident: HartLocal<#ty> = unsafe { HartLocal::new(&#backing_ident) };
    };

    Ok(expanded)
}
