use {
    proc_macro2::TokenStream,
    quote::quote,
    std::{collections::HashMap, env, fs, path::Path},
};

const PSF2_MAGIC: u32 = 0x864a_b572;
const PSF2_HAS_UNICODE_TABLE: u32 = 0x01;
const PSF2_SEPARATOR: u8 = 0xFF;
const PSF2_STARTSEQ: u8 = 0xFE;

struct Psf2Header {
    header_size: u32,
    flags: u32,
    glyph_count: u32,
    bytes_per_glyph: u32,
    height: u32,
    width: u32,
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn parse_header(data: &[u8]) -> Psf2Header {
    assert!(data.len() >= 32, "Font file too small to contain PSF2 header");
    let magic = read_u32_le(data, 0);
    assert_eq!(magic, PSF2_MAGIC, "Not a PSF2 font (bad magic)");
    Psf2Header {
        header_size: read_u32_le(data, 8),
        flags: read_u32_le(data, 12),
        glyph_count: read_u32_le(data, 16),
        bytes_per_glyph: read_u32_le(data, 20),
        height: read_u32_le(data, 24),
        width: read_u32_le(data, 28),
    }
}

fn parse_unicode_table(data: &[u8], header: &Psf2Header) -> HashMap<u16, Vec<Vec<u8>>> {
    let glyph_data_size = header.glyph_count as usize * header.bytes_per_glyph as usize;
    let table_start = header.header_size as usize + glyph_data_size;
    assert!(
        (header.flags & PSF2_HAS_UNICODE_TABLE) != 0,
        "Font has no Unicode table"
    );
    assert!(
        data.len() > table_start,
        "Font file truncated before Unicode table"
    );

    let table = &data[table_start..];
    let mut map: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    let mut glyph_index: u16 = 0;
    let mut pos = 0;

    while pos < table.len() && (glyph_index as u32) < header.glyph_count {
        let mut current_seq: Vec<u8> = Vec::new();
        let mut in_ligature = false;

        while pos < table.len() {
            let byte = table[pos];
            pos += 1;

            if byte == PSF2_SEPARATOR {
                if !current_seq.is_empty() && !in_ligature {
                    map.entry(glyph_index).or_default().push(current_seq.clone());
                }
                current_seq.clear();
                in_ligature = false;
                break;
            } else if byte == PSF2_STARTSEQ {
                if !current_seq.is_empty() && !in_ligature {
                    map.entry(glyph_index).or_default().push(current_seq.clone());
                }
                current_seq.clear();
                in_ligature = true;
            } else {
                current_seq.push(byte);

                if pos < table.len() {
                    let next = table[pos];
                    let next_is_boundary = next == PSF2_SEPARATOR
                        || next == PSF2_STARTSEQ
                        || (next & 0x80 == 0)
                        || (next & 0xC0 == 0xC0);
                    if next_is_boundary && is_complete_utf8(&current_seq) && !in_ligature {
                        map.entry(glyph_index).or_default().push(current_seq.clone());
                        current_seq.clear();
                    }
                }
            }
        }

        if !current_seq.is_empty() && !in_ligature && is_complete_utf8(&current_seq) {
            map.entry(glyph_index).or_default().push(current_seq.clone());
        }

        glyph_index += 1;
    }

    map
}

fn is_complete_utf8(seq: &[u8]) -> bool {
    if seq.is_empty() {
        return false;
    }
    seq.len() == utf8_seq_len(seq[0])
}

fn utf8_seq_len(first_byte: u8) -> usize {
    if first_byte & 0x80 == 0 {
        1
    } else if first_byte & 0xE0 == 0xC0 {
        2
    } else if first_byte & 0xF0 == 0xE0 {
        3
    } else if first_byte & 0xF8 == 0xF0 {
        4
    } else {
        1
    }
}

struct State {
    transitions: [Option<u16>; 256],
    output: Option<u16>,
}

impl State {
    fn new() -> Self {
        Self {
            transitions: [None; 256],
            output: None,
        }
    }
}

fn build_trie(utf8_to_glyph: &[(Vec<u8>, u16)]) -> Vec<State> {
    let mut states: Vec<State> = vec![State::new()];

    for (seq, glyph_index) in utf8_to_glyph {
        let mut current = 0usize;
        for (i, &byte) in seq.iter().enumerate() {
            let is_last = i == seq.len() - 1;
            let next = if let Some(next) = states[current].transitions[byte as usize] {
                next as usize
            } else {
                let next = states.len();
                assert!(next <= u16::MAX as usize, "Too many trie states");
                states.push(State::new());
                states[current].transitions[byte as usize] = Some(next as u16);
                next
            };
            if is_last && states[next].output.is_none() {
                states[next].output = Some(*glyph_index);
            }
            current = next;
        }
    }

    states
}

fn generate_state_machine(states: &[State]) -> TokenStream {
    let num_states = states.len();

    let transition_rows: Vec<TokenStream> = states
        .iter()
        .map(|state| {
            let entries: Vec<TokenStream> = state
                .transitions
                .iter()
                .map(|t| match t {
                    Some(next) => quote! { Some(#next) },
                    None => quote! { None },
                })
                .collect();
            quote! { [#(#entries),*] }
        })
        .collect();

    let output_entries: Vec<TokenStream> = states
        .iter()
        .map(|state| match state.output {
            Some(g) => quote! { Some(#g) },
            None => quote! { None },
        })
        .collect();

    quote! {
        const STATE_TRANSITIONS: [[Option<u16>; 256]; #num_states] = [
            #(#transition_rows),*
        ];

        const STATE_OUTPUTS: [Option<u16>; #num_states] = [
            #(#output_entries),*
        ];
    }
}

fn generate_glyph_data(data: &[u8], header: &Psf2Header) -> TokenStream {
    let glyph_count = header.glyph_count as usize;
    let bytes_per_glyph = header.bytes_per_glyph as usize;
    let height = header.height as usize;
    let width = header.width as usize;
    let bytes_per_row = width.div_ceil(8);
    let header_size = header.header_size as usize;

    assert_eq!(
        bytes_per_row * height,
        bytes_per_glyph,
        "bytes_per_glyph doesn't match height * bytes_per_row"
    );

    let glyphs: Vec<TokenStream> = (0..glyph_count)
        .map(|g| {
            let start = header_size + g * bytes_per_glyph;
            let glyph_bytes = &data[start..start + bytes_per_glyph];

            let rows: Vec<TokenStream> = (0..height)
                .map(|row| {
                    let row_bytes = &glyph_bytes[row * bytes_per_row..(row + 1) * bytes_per_row];
                    quote! { [#(#row_bytes),*] }
                })
                .collect();

            quote! { [#(#rows),*] }
        })
        .collect();

    quote! {
        pub const GLYPH_WIDTH: usize = #width;
        pub const GLYPH_HEIGHT: usize = #height;
        pub const GLYPH_BYTES_PER_ROW: usize = #bytes_per_row;
        pub const GLYPH_COUNT: usize = #glyph_count;

        pub static GLYPHS: [[[u8; GLYPH_BYTES_PER_ROW]; GLYPH_HEIGHT]; GLYPH_COUNT] = [
            #(#glyphs),*
        ];
    }
}

fn generate_feed_fn(fallback_glyph: u16) -> TokenStream {
    quote! {
        impl FontStateMachine {
            pub fn feed(&mut self, byte: u8) -> GlyphResult {
                match STATE_TRANSITIONS[self.state as usize][byte as usize] {
                    None => {
                        self.state = 0;
                        GlyphResult::Fallback(#fallback_glyph)
                    }
                    Some(next) => {
                        self.state = next;
                        match STATE_OUTPUTS[next as usize] {
                            Some(glyph) => {
                                self.state = 0;
                                GlyphResult::Found(glyph)
                            }
                            None => GlyphResult::Incomplete,
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let font_path = Path::new(&manifest_dir).join("../../run/font.psf");

    println!("cargo:rerun-if-changed={}", font_path.display());

    let data = fs::read(&font_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read font file at {}: {e}\n\
             Run `make font` to download the font before building.",
            font_path.display()
        )
    });

    let header = parse_header(&data);

    let glyph_to_seqs = parse_unicode_table(&data, &header);
    let mut utf8_to_glyph: Vec<(Vec<u8>, u16)> = glyph_to_seqs
        .into_iter()
        .flat_map(|(glyph_idx, seqs)| seqs.into_iter().map(move |seq| (seq, glyph_idx)))
        .collect();
    utf8_to_glyph.sort_by(|a, b| a.0.cmp(&b.0));

    let replacement_utf8: Vec<u8> = vec![0xEF, 0xBF, 0xBD];
    let fallback_glyph = utf8_to_glyph
        .iter()
        .find(|(seq, _)| seq == &replacement_utf8)
        .map(|(_, idx)| *idx)
        .unwrap_or(0);

    let states = build_trie(&utf8_to_glyph);

    let out_dir = env::var("OUT_DIR").unwrap();

    let sm_tokens = generate_state_machine(&states);
    fs::write(
        Path::new(&out_dir).join("state_machine.rs"),
        prettyplease::unparse(&syn::parse2(sm_tokens).unwrap()),
    )
    .unwrap();

    let glyph_tokens = generate_glyph_data(&data, &header);
    fs::write(
        Path::new(&out_dir).join("glyphs.rs"),
        prettyplease::unparse(&syn::parse2(glyph_tokens).unwrap()),
    )
    .unwrap();

    let feed_tokens = generate_feed_fn(fallback_glyph);
    fs::write(
        Path::new(&out_dir).join("feed.rs"),
        prettyplease::unparse(&syn::parse2(feed_tokens).unwrap()),
    )
    .unwrap();
}