use builder::{Config, Target};

fn main() {
    builder::configure(Config {
        target: Target::PE32P,
    });
    builder::build_nasm_flat("src/hart/ap_init.asm", "ap_init.bin");
}