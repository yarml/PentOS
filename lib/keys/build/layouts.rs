use {
    proc_macro2::TokenStream,
    quote::{format_ident, quote},
    serde::Deserialize,
    syn::Ident,
};

#[derive(Deserialize)]
struct LayoutFile {
    name: Option<String>,
    id: Option<usize>,
    alias: Option<Alias>,
    layout: Option<Layout>,
}

#[derive(Deserialize)]
struct Alias {
    target: String,
}

#[derive(Deserialize)]
struct Layout {
    name: String,
    inherit: Option<String>,
    key: Option<Vec<KeyDef>>,
    #[serde(rename = "dead-key")]
    dead_key: Option<Vec<DeadKeyDef>>,
}

#[derive(Deserialize)]
struct KeyDef {
    name: String,
    normal: char,
    shifted: char,
    alt: Option<char>,
    #[serde(rename = "shifted-alt")]
    shifted_alt: Option<char>,
}

#[derive(Deserialize)]
struct DeadKeyDef {
    name: String,
    normal: char,
    shifted: char,
}

fn transform(layouts: &[LayoutFile]) -> TokenStream {
    let layouts_def = layouts.iter().map(|layout| {
        let idname = layout.id_name();
        let name = layout.instance_name();

        quote! {
            pub const #name: Layout = Layout::of_id(#idname);
        }
    });

    let ids_def = layouts.iter().map(|layout| {
        let idname = layout.id_name();
        let id = layout.id.unwrap();

        quote! {
            const #idname: usize = #id;
        }
    });

    let resolve_fns = layouts.iter().map(|layout| {
        let fn_name = layout.resolve_fn_name();

        let body = if let Some(alias) = layout.alias.as_ref() {
            let target = layouts
                .iter()
                .find(|l| *l.name.as_ref().unwrap() == alias.target)
                .unwrap_or_else(|| {
                    panic!(
                        "could not find aliased layout: {}. While parsing layout {}",
                        alias.target,
                        layout.name.as_ref().unwrap()
                    )
                });
            let aliased_resolve_fn = target.resolve_fn_name();
            quote! {
                #aliased_resolve_fn(key)
            }
        } else {
            let keys = layout.layout.as_ref().unwrap().key.as_ref();

            let keys_match_cases = keys
                .map(|keys| {
                    keys.iter().map(|k| {
                        let normal = k.normal;
                        let shifted = k.shifted;
                        let alt = k.alt.unwrap_or(normal);
                        let shifted_alt = k.shifted_alt.unwrap_or(shifted);

                        let instance_name = format_ident!("KEY_{}", k.name);

                        quote! {
                            crate::#instance_name => Some(Character::new(#normal, #shifted, #alt, #shifted_alt)),
                        }
                    }).collect()
                })
                .unwrap_or(vec![]);

            quote! {
                match key {
                    #(#keys_match_cases)*
                    _ => None,
                }
            }
        };

        quote! {
            fn #fn_name(key: Key) -> Option<Character> {
                #body
            }
        }
    });

    let resolve_specific_match_cases = layouts.iter().map(|layout| {
        let idname = layout.id_name();
        let fn_name = layout.resolve_fn_name();

        quote! {
            #idname => #fn_name(key),
        }
    });

    quote! {

        #(#layouts_def)*

        #(#ids_def)*
        impl Layout {
            fn resolve_specific(&self, key: Key) -> Option<Character> {
                match self.id {
                    #(#resolve_specific_match_cases)*
                    _ => unreachable!(),
                }
            }
        }

        #(#resolve_fns)*

    }
}

pub fn generate() {
    let dir = builder::load_dir("layouts");

    let layouts: Vec<LayoutFile> = dir
        .enumerate()
        .filter_map(|(i, f)| {
            let f = f.unwrap();
            if !f.file_type().unwrap().is_file() {
                return None;
            }
            let fname = f.file_name().to_string_lossy().to_string();
            if !fname.ends_with(".toml") {
                return None;
            }

            let mut layout_file: LayoutFile =
                toml::from_str(&builder::load_file(format!("layouts/{fname}")))
                    .unwrap_or_else(|e| panic!("Failed to parse: layouts/{fname}: {e}"));

            if layout_file.name.is_none() {
                layout_file.name = Some(fname.split(".").next().unwrap().to_string());
            }

            layout_file.id = Some(i);

            Some(layout_file)
        })
        .collect();

    builder::generate_rs("layouts.rs", transform(&layouts));
}

impl LayoutFile {
    fn instance_name(&self) -> Ident {
        format_ident!("LAYOUT_{}", self.name.as_ref().unwrap().to_uppercase())
    }
    fn id_name(&self) -> Ident {
        format_ident!("ID_LAYOUT_{}", self.name.as_ref().unwrap().to_uppercase())
    }
    fn resolve_fn_name(&self) -> Ident {
        format_ident!("resolve_{}", self.name.as_ref().unwrap())
    }
}
