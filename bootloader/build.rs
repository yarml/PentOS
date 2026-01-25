use builder::Config;
use builder::Target;

fn main() {
    builder::configure(Config {
        target: Target::PE32P,
    });
    builder::add_nasm_lib("hart", &["src/hart/ap_init.asm"]);
}