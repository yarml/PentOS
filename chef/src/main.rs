#![feature(exit_status_error)]

mod args;
mod config;
mod progress;

use {
    args::{ChefArgs, ChefCommand},
    cargo_metadata::{Metadata, MetadataCommand},
    clap::Parser,
    config::ChefConfig,
    serde_json::Value,
    std::{fs, io::Read, process::exit},
    tar::Archive,
    xz::read::XzDecoder,
};

fn packages(root: &Metadata) {
    for package in &root.workspace_members {
        let package = &root[package];
        println!("{}", package.name);
    }
    exit(0);
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
    if let Some(config) = raw_config[name].as_str() {
        print!("{config}");
        exit(0);
    } else {
        exit(1);
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
        ChefCommand::Packages => {
            packages(&root);
        }
        ChefCommand::Config { name } => {
            printconfig(raw_config, &name);
        }
    }
}
