mod ps2;

use {
    proc_macro2::TokenStream,
    ps2::{KeySeq, KeyType},
    quote::{format_ident, quote},
    serde::Deserialize,
    std::{env, fs, path::Path},
};

#[derive(Deserialize)]
struct KeysFile {
    key: Vec<KeyDef>,
}

#[derive(Deserialize)]
struct KeyDef {
    name: String,
    #[serde(rename = "ps2-type")]
    ps2_type: KeyType,
    #[serde(rename = "ps2-scancode")]
    ps2_scancode: Option<u8>,
}

fn generate_keys(keys: &[(usize, String, KeySeq)]) -> TokenStream {
    let consts: Vec<TokenStream> = keys
        .iter()
        .map(|(id, name, _)| {
            let ident = format_ident!("KEY_{}", name);
            quote! { pub const #ident: Key = Key::of_id(#id); }
        })
        .collect();

    let match_cases: Vec<TokenStream> = keys
        .iter()
        .map(|(id, name, _)| {
            let ident = format_ident!("KEY_{}", name);
            quote! { #id => stringify!(#ident) }
        })
        .collect();

    let keys_count: usize = keys.len();

    quote! {
        #(#consts)*

        pub const KEYS_COUNT: usize = #keys_count;

        impl core::fmt::Debug for Key {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let name = match self.id {
                    #(#match_cases),*,
                    _ => unreachable!()
                };
                write!(f, "{}", name)
            }
        }
    }
}

pub fn generate() {
    let keys_file: KeysFile =
        toml::from_str(&builder::load_file("keys.toml")).expect("Failed to parse keys.toml");

    let keys: Vec<(usize, String, KeySeq)> = keys_file
        .key
        .into_iter()
        .enumerate()
        .map(|(id, def)| {
            let seq = match def.ps2_type {
                KeyType::Simple => {
                    KeySeq::Simple(def.ps2_scancode.expect("simple key requires scancode"))
                }
                KeyType::Extended => {
                    KeySeq::Extended(def.ps2_scancode.expect("extended key requires scancode"))
                }
                KeyType::PrintScreen => KeySeq::PrintScreen,
                KeyType::Pause => KeySeq::Pause,
            };
            (id, def.name, seq)
        })
        .collect();

    let out_dir = env::var("OUT_DIR").unwrap();

    let keys_tokens = generate_keys(&keys);
    fs::write(
        Path::new(&out_dir).join("keys.rs"),
        prettyplease::unparse(&syn::parse2(keys_tokens).unwrap()),
    )
    .unwrap();

    let ps2_sm_tokens = ps2::generate_state_machine(&keys);
    fs::write(
        Path::new(&out_dir).join("ps2_state_machine.rs"),
        prettyplease::unparse(&syn::parse2(ps2_sm_tokens).unwrap()),
    )
    .unwrap();
}
