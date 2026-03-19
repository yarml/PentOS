mod args;
mod config;
mod progress;

use {
    crate::{args::PackagesCommand, config::resolution::Resolution},
    args::{ChefArgs, ChefCommand},
    cargo_metadata::{Metadata, MetadataCommand},
    clap::Parser,
    config::ChefConfig,
    serde_json::Value,
    std::{fs, io::Read, process::exit},
    tar::Archive,
    xz::read::XzDecoder,
};

fn packages_names(root: &Metadata) {
    for package in &root.workspace_members {
        let package = &root[package];
        println!("{}", package.name);
    }
    exit(0);
}

fn packages_paths(root: &Metadata) {
    for package in &root.workspace_members {
        let package = &root[package];
        let mut path = package.manifest_path.clone();
        path.pop();
        println!("{}", path);
    }
    exit(0);
}

fn packages_userbin_names(root: &Metadata) {
    for package in &root.workspace_members {
        let package = &root[package];
        let mut path = package.manifest_path.clone();
        path.pop(); // Cargo.toml
        let Some(name) = path.file_name().map(String::from) else {
            continue;
        };
        path.pop();
        let Some(package) = path.file_name().map(String::from) else {
            continue;
        };
        path.pop();
        let Some(userdir) = path.file_name().map(String::from) else {
            continue;
        };
        if package == "lib" {
            continue;
        }
        if userdir != "user" {
            continue;
        }

        println!("{}/{}", package, name);
    }
}

fn ovmf(config: &ChefConfig) {
    let source = config
        .ovmf_source_template
        .replace("$$", &config.ovmf_version);
    let archive_varsfd_path = config
        .ovmf_varsfd_path_template
        .replace("$$", &config.ovmf_version);
    let archive_codefd_path = config
        .ovmf_codefd_path_template
        .replace("$$", &config.ovmf_version);

    print_action!(0, "Setting up", "OVMF",);
    print_action!(1, "Downloading", "OVMF ({source})",);
    let ovmf_tarball = reqwest::blocking::get(&source)
        .expect("Couldn't download OVMF tarball")
        .bytes()
        .expect("Couldn't read OVMF tarball");
    print_action!(1, "Decompressing", "OVMF");
    let mut decompressor = XzDecoder::new(ovmf_tarball.as_ref());
    let mut decompressed = Vec::new();
    decompressor
        .read_to_end(&mut decompressed)
        .expect("Couldn't decompress OVMF tarball");
    let mut archive = Archive::new(decompressed.as_slice());

    let root_path = "run/ovmf";
    let varsfd_path = format!("{root_path}/vars.fd");
    let codefd_path = format!("{root_path}/code.fd");

    fs::create_dir_all(root_path).unwrap();
    for entry in archive.entries().expect("Couldn't read OVMF tarball") {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_str().unwrap().to_string();
        if path == archive_varsfd_path {
            print_action!(1, "Installing", "OVMF_VARS ({varsfd_path})");
            let mut file = fs::File::create(&varsfd_path).unwrap();
            std::io::copy(&mut entry, &mut file).unwrap();
        }
        if path == archive_codefd_path {
            print_action!(1, "Installing", "OVMF_CODE ({codefd_path})");
            let mut file = fs::File::create(&codefd_path).unwrap();
            std::io::copy(&mut entry, &mut file).unwrap();
        }
    }
}

fn printconfig(raw_config: &Value, name: &str) {
    let cfg = match name {
        "qemu-xres" | "qemu-yres" | "qemu-vgamem_mb" => {
            let Some(raw_resolution) = raw_config["qemu-resolution"].as_str() else {
                eprintln!("qemu-resolution not specified in build configuration");
                exit(1);
            };
            let Ok(resolution) = Resolution::try_from(raw_resolution) else {
                eprintln!("invalid qemu-resolution in build configuration");
                exit(1);
            };

            if name == "qemu-xres" {
                Some(format!("{}", resolution.xres))
            } else if name == "qemu-yres" {
                Some(format!("{}", resolution.yres))
            } else if name == "qemu-vgamem_mb" {
                let vgamem_mb = (resolution.xres * resolution.yres * 4).div_ceil(1024 * 1024);
                Some(format!("{}", vgamem_mb))
            } else {
                unreachable!()
            }
        }
        _ => raw_config[name].as_str().map(|v| v.to_string()),
    };
    match cfg {
        Some(cfg) => {
            print!("{cfg}");
            exit(0);
        }
        None => exit(1),
    }
}

fn main() {
    let args = ChefArgs::parse();
    let root = MetadataCommand::new()
        .exec()
        .expect("Couldn't get Cargo metadata");
    let raw_config = &root.workspace_metadata["chef"];
    let config = ChefConfig::from(&root.workspace_metadata["chef"]);
    match args.command {
        ChefCommand::Ovmf => {
            ovmf(&config);
        }
        ChefCommand::Packages { command } => match command {
            PackagesCommand::Name => packages_names(&root),
            PackagesCommand::Path => packages_paths(&root),
            PackagesCommand::Userbin => packages_userbin_names(&root),
        },
        ChefCommand::Config { name } => {
            printconfig(raw_config, &name);
        }
    }
}
