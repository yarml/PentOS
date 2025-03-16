mod args;
mod cargo;
mod config;
mod progress;
mod qemu;
mod utils;

use args::ChefArgs;
use args::ChefCommand;
use cargo::Cargo;
use cargo::Packages;
use cargo_metadata::Metadata;
use cargo_metadata::MetadataCommand;
use clap::Parser;
use config::ChefConfig;
use qemu::Drive;
use qemu::DriveInterface;
use qemu::Qemu;
use std::fs;
use std::io::Read;
use std::process;
use tar::Archive;
use utils::get_path;
use xz::read::XzDecoder;

fn check(root: &Metadata) {
    for package in &root.workspace_members {
        let package = &root[package];
        let check_cmd = Cargo::check()
            .keep_going()
            .quiet()
            .message_format("json")
            .packages(Packages::packages(&[&package.id.repr]));
        let path = get_path(package);
        check_cmd
            .command()
            // Change PWD to use package's .cargo/config.toml
            .current_dir(path)
            .status()
            .unwrap();
    }
}

fn build(root: &Metadata, target_package_name: &str) {
    let mut target_package = None;

    for package in &root.workspace_members {
        let package = &root[package];
        if package.name == target_package_name {
            target_package = Some(package);
            break;
        }
    }

    if target_package.is_none() {
        print_error!("Couldn't find package {target_package_name}");
        process::exit(1);
    }

    let target_package = target_package.unwrap();

    let build_cmd = Cargo::build().packages(Packages::packages(&[&target_package.id.repr]));
    let path = get_path(target_package);
    build_cmd
        .command()
        // Change PWD to use package's .cargo/config.toml
        .current_dir(path)
        .status()
        .unwrap();
}

fn ovmf(config: &ChefConfig) {
    print_action!(0, "Setting up", "OVMF",);
    print_action!(
        1,
        "Downloading",
        "OVMF ({source})",
        source = config.ovmf_source
    );
    let ovmf_tarball = reqwest::blocking::get(&config.ovmf_source)
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
    fs::create_dir_all("run/ovmf").unwrap();
    for entry in archive.entries().expect("Couldn't read OVMF tarball") {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_str().unwrap().to_string();
        if path == "edk2-stable202408.01-r1-bin/x64/vars.fd" {
            print_action!(1, "Installing", "OVMF_VARS (run/ovmf/vars.fd)");
            let mut file = fs::File::create("run/ovmf/vars.fd").unwrap();
            std::io::copy(&mut entry, &mut file).unwrap();
        }
        if path == "edk2-stable202408.01-r1-bin/x64/code.fd" {
            print_action!(1, "Installing", "OVMF_CODE (run/ovmf/code.fd)");
            let mut file = fs::File::create("run/ovmf/code.fd").unwrap();
            std::io::copy(&mut entry, &mut file).unwrap();
        }
    }
}

fn image(root: &Metadata) {
    build(root, "bootloader");
    fs::create_dir_all("run/esp/efi/boot").unwrap();
    fs::copy(
        "target/x86_64-unknown-uefi/debug/bootloader.efi",
        "run/esp/efi/boot/bootx64.efi",
    )
    .unwrap();
}

fn run(root: &Metadata) {
    image(root);
    let qemu = Qemu::new()
        .numcores(4)
        .memory(qemu::Memory::Giga(4))
        .debugcon("stdio")
        .drive(
            Drive::new("run/ovmf/code.fd")
                .interface(DriveInterface::Pflash)
                .raw()
                .readonly(),
        )
        .drive(
            Drive::new("run/ovmf/vars.fd")
                .interface(DriveInterface::Pflash)
                .raw()
                .readonly(),
        )
        .drive(Drive::new("fat:rw:run/esp").raw());
    qemu.command().status().unwrap();
}

fn main() {
    let args = ChefArgs::parse();
    let root = MetadataCommand::new()
        .exec()
        .expect("Couldn't get Cargo metadata");
    let config = ChefConfig::from(&root.workspace_metadata["chef"]);
    match args.command {
        ChefCommand::Check => {
            check(&root);
        }
        ChefCommand::Build { package } => {
            build(&root, &package);
        }
        ChefCommand::Image => {
            image(&root);
        }
        ChefCommand::Run => {
            run(&root);
        }
        ChefCommand::Ovmf => {
            ovmf(&config);
        }
    }
}
