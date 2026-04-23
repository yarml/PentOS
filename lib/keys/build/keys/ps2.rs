use {
    proc_macro2::TokenStream,
    quote::{format_ident, quote},
    serde::Deserialize,
    std::collections::HashMap,
};

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KeyType {
    Simple,
    Extended,
    PrintScreen,
    Pause,
}

#[derive(Clone)]
pub enum KeySeq {
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

pub fn generate_state_machine(keys: &[(usize, String, KeySeq)]) -> TokenStream {
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
                    quote! { Some(KeyEvent::Tap(crate::#key_ident)) }
                } else if *is_press {
                    quote! { Some(KeyEvent::Pressed(crate::#key_ident)) }
                } else {
                    quote! { Some(KeyEvent::Released(crate::#key_ident)) }
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
