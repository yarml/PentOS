use std::{
    env,
    fs::{self, ReadDir},
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use proc_macro2::TokenStream;

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
    if assemblies.is_empty() {
        return;
    }

    let cfg = getcfg();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    for assembly in assemblies {
        println!("cargo::rerun-if-changed={assembly}");
    }

    let mut objects = Vec::new();
    for assembly in assemblies {
        let src = PathBuf::from(assembly);
        let obj = out_dir.join(src.file_stem().unwrap()).with_extension("o");

        let mut cmd = Command::new("nasm");
        cmd.args(cfg.nasm_flags());
        cmd.args(["-o", obj.to_str().unwrap(), assembly]);

        let status = cmd.status().expect("failed to run nasm - is it in PATH?");
        if !status.success() {
            panic!("NASM failed to assemble: {}", assembly);
        }
        objects.push(obj);
    }

    let lib_path = out_dir.join(format!("lib{}.a", libname));
    let mut ar = Command::new("llvm-ar");
    ar.arg("crs").arg(&lib_path);
    for obj in &objects {
        ar.arg(obj);
    }

    let status = ar.status().expect("failed to run llvm-ar");
    if !status.success() {
        panic!("llvm-ar failed to create library: {}", libname);
    }

    println!("cargo::rustc-link-search={}", out_dir.display());
    println!("cargo::rustc-link-lib={libname}");
}

pub fn build_nasm_flat(src: &str, bin: &str) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_dir.join(bin);

    let status = Command::new("nasm")
        .args(["-f", "bin", "-o", out_file.to_str().unwrap(), src])
        .status()
        .expect("failed to run nasm - is it in PATH?");

    if !status.success() {
        panic!("NASM failed to assemble flat binary");
    }
    println!("cargo:rerun-if-changed={src}");
}

/// Load a file relative to crate root
pub fn load_file<P: AsRef<Path>>(path: P) -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let file = Path::new(&manifest_dir).join(path);

    println!("cargo:rerun-if-changed={}", file.display());

    fs::read_to_string(&file).unwrap_or_else(|_| panic!("Failed to read {}", file.display()))
}

pub fn load_dir<P: AsRef<Path>>(path: P) -> ReadDir {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = Path::new(&manifest_dir).join(path);

    let dirname = dir.display().to_string();

    fs::read_dir(dir).unwrap_or_else(|_| panic!("Failed to open directory {dirname}"))
}

pub fn generate_rs(name: &str, tokens: TokenStream) {
    let out_dir = env::var("OUT_DIR").unwrap();
    fs::write(
        Path::new(&out_dir).join(name),
        prettyplease::unparse(&syn::parse2(tokens).unwrap()),
    )
    .unwrap();
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
            Target::Elf64 => vec!["-felf64", "-gdwarf"],
            Target::PE32P => vec!["-fwin64", "-gcv8"],
        }
    }
}
