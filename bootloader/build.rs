use builder::Config;
use builder::Target;

fn main() {
    builder::configure(Config {
        target: Target::PE32P,
    });
    builder::add_nasm_lib("mp", &["src/mp/ap_init.asm"]);
}
