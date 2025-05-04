use builder::Config;
use builder::Target;

fn main() {
    builder::configure(Config {
        target: Target::Elf64,
    });
    let assemblies = [];
    builder::add_nasm_lib("pent-kernel-asm", &assemblies);
}
