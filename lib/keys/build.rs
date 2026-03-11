use {
    proc_macro2::TokenStream,
    quote::{format_ident, quote},
    serde::Deserialize,
    std::{collections::HashMap, env, fs, path::Path},
};

#[derive(Deserialize)]
struct KeysFile {
    key: Vec<KeyDef>,
}

#[derive(Deserialize)]
struct KeyDef {
    name: String,
    #[serde(rename = "type")]
    ty: KeyType,
    scancode: Option<u8>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum KeyType {
    Simple,
    Extended,
    PrintScreen,
    Pause,
}

#[derive(Clone)]
enum KeySeq {
    Simple(u8),
    Extended(u8),
    PrintScreen,
    Pause,
}

impl KeySeq {
    fn press_sequence(&self) -> Vec<u8> {
        match self {
            KeySeq::Simple(s) => vec![*s],
            KeySeq::Extended(s) => vec![0xE0, *s],
            KeySeq::PrintScreen => vec![0xE0, 0x12, 0xE0, 0x7C],
            KeySeq::Pause => vec![0xE1, 0x14, 0x77, 0xE1, 0xF0, 0x14, 0xF0, 0x77],
        }
    }

    fn release_sequence(&self) -> Option<Vec<u8>> {
        match self {
            KeySeq::Simple(s) => Some(vec![0xF0, *s]),
            KeySeq::Extended(s) => Some(vec![0xE0, 0xF0, *s]),
            KeySeq::PrintScreen => Some(vec![0xE0, 0xF0, 0x7C, 0xE0, 0xF0, 0x12]),
            KeySeq::Pause => None,
        }
    }
}

struct State {
    transitions: HashMap<u8, usize>,
    output: Option<(String, bool, bool)>,
}

impl State {
    fn new() -> Self {
        Self {
            transitions: HashMap::new(),
            output: None,
        }
    }
}

fn insert_sequence(
    states: &mut Vec<State>,
    seq: &[u8],
    key_name: &str,
    is_press: bool,
    is_tap: bool,
) {
    let mut current = 0;
    for &byte in seq {
        let next = if let Some(&next) = states[current].transitions.get(&byte) {
            next
        } else {
            let next = states.len();
            states.push(State::new());
            states[current].transitions.insert(byte, next);
            next
        };
        current = next;
    }
    states[current].output = Some((key_name.to_string(), is_press, is_tap));
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

fn generate_state_machine(keys: &[(usize, String, KeySeq)]) -> TokenStream {
    let mut states: Vec<State> = vec![State::new()];

    for (_, name, seq) in keys {
        let release_seq = seq.release_sequence();
        insert_sequence(
            &mut states,
            &seq.press_sequence(),
            name,
            true,
            release_seq.is_none(),
        );
        if let Some(release) = release_seq {
            insert_sequence(&mut states, &release, name, false, false);
        }
    }

    let num_states = states.len();

    let transition_tables: Vec<TokenStream> = states
        .iter()
        .map(|state| {
            let entries: Vec<TokenStream> = (0u16..=255)
                .map(|byte| {
                    if let Some(&next) = state.transitions.get(&(byte as u8)) {
                        let next = next as u16;
                        quote! { Some(#next) }
                    } else {
                        quote! { None }
                    }
                })
                .collect();
            quote! { [#(#entries),*] }
        })
        .collect();

    let outputs: Vec<TokenStream> = states
        .iter()
        .map(|state| {
            if let Some((key_name, is_press, is_tap)) = &state.output {
                let key_ident = format_ident!("KEY_{}", key_name);
                if *is_tap {
                    quote! { Some(KeyEvent::Tap(#key_ident)) }
                } else if *is_press {
                    quote! { Some(KeyEvent::Pressed(#key_ident)) }
                } else {
                    quote! { Some(KeyEvent::Released(#key_ident)) }
                }
            } else {
                quote! { None }
            }
        })
        .collect();

    quote! {
        const STATE_TRANSITIONS: [[Option<u16>; 256]; #num_states] = [
            #(#transition_tables),*
        ];

        const STATE_OUTPUTS: [Option<KeyEvent>; #num_states] = [
            #(#outputs),*
        ];
    }
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let keys_toml = Path::new(&manifest_dir).join("keys.toml");

    println!("cargo:rerun-if-changed={}", keys_toml.display());

    let content = fs::read_to_string(&keys_toml).expect("Failed to read keys.toml");
    let keys_file: KeysFile = toml::from_str(&content).expect("Failed to parse keys.toml");

    let keys: Vec<(usize, String, KeySeq)> = keys_file
        .key
        .into_iter()
        .enumerate()
        .map(|(id, def)| {
            let seq = match def.ty {
                KeyType::Simple => {
                    KeySeq::Simple(def.scancode.expect("simple key requires scancode"))
                }
                KeyType::Extended => {
                    KeySeq::Extended(def.scancode.expect("extended key requires scancode"))
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

    let sm_tokens = generate_state_machine(&keys);
    fs::write(
        Path::new(&out_dir).join("state_machine.rs"),
        prettyplease::unparse(&syn::parse2(sm_tokens).unwrap()),
    )
    .unwrap();
}
