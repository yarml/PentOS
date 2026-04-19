use std::{env, path::PathBuf, process::Command, sync::Mutex};

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