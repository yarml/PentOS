use std::sync::Mutex;

static CONFIG: Mutex<Config> = Mutex::new(Config {
    target: Target::Elf64,
});

#[derive(Clone, Copy)]
pub struct Config {
    pub target: Target,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Elf64,
    PE32P,
}

pub fn add_nasm_lib(libname: &str, assemblies: &[&str]) {
    let cfg = getcfg();
    for assembly in assemblies {
        println!("cargo::rerun-if-changed={assembly}");
    }
    let full_lib_name = format!("lib{}.a", libname);
    nasm_rs::compile_library_args(&full_lib_name, assemblies, &cfg.nasm_flags())
        .expect("Could not compile NASM library");
    println!("cargo::rustc-link-lib={libname}");
}

pub fn configure(config: Config) {
    let mut builder_config = CONFIG.lock().unwrap();
    *builder_config = config;
}

fn getcfg() -> Config {
    *CONFIG.lock().unwrap()
}

impl Config {
    fn nasm_flags(&self) -> Vec<&'static str> {
        match self.target {
            Target::Elf64 => vec!["-felf64"],
            Target::PE32P => vec!["-fwin64"],
        }
    }
}
