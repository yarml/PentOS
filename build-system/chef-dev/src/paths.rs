use {
    crate::args::{BuildProfile, GeneratePartition},
    chef_core::{crates::Pkg, paths::normalize_path},
    std::path::PathBuf,
};

const BUILD_DIR: &str = ".build";

pub fn build_dir(profile: BuildProfile) -> String {
    format!("{BUILD_DIR}/{}", profile.pathname())
}

pub fn varsfd() -> PathBuf {
    let rel_path = format!("{BUILD_DIR}/ovmf/vars.fd");
    normalize_path(rel_path)
}
pub fn codefd() -> PathBuf {
    let rel_path = format!("{BUILD_DIR}/ovmf/code.fd");
    normalize_path(rel_path)
}
pub fn font() -> PathBuf {
    let rel_path = format!("{BUILD_DIR}/font.psf");
    normalize_path(rel_path)
}

pub fn target_kernel(profile: BuildProfile) -> PathBuf {
    let rel_path = format!("target/kernel/{}/kernel", profile.pathname());
    normalize_path(rel_path)
}

pub fn target_bootloader(profile: BuildProfile) -> PathBuf {
    let rel_path = format!("target/uefi/{}/bootloader.efi", profile.pathname());
    normalize_path(rel_path)
}

pub fn target_pkg(name: &str, profile: BuildProfile) -> PathBuf {
    let rel_path = format!("target/user/{}/{name}", profile.pathname());
    normalize_path(rel_path)
}

pub fn flat_dir(partition: GeneratePartition, profile: BuildProfile) -> String {
    format!(
        "{}/flat/{}",
        build_dir(profile),
        match partition {
            GeneratePartition::Boot => "boot",
            GeneratePartition::System => "system",
            GeneratePartition::Disk => panic!("disk configuration does not have a flat directory"),
        }
    )
}
pub fn img_dir(profile: BuildProfile) -> String {
    format!("{}/img", build_dir(profile))
}

pub fn flat_kernel(profile: BuildProfile) -> PathBuf {
    let rel_path = format!(
        "{}/sys/kernel",
        flat_dir(GeneratePartition::System, profile)
    );
    normalize_path(rel_path)
}

pub fn flat_bootloader(profile: BuildProfile) -> PathBuf {
    let rel_path = format!(
        "{}/efi/boot/bootx64.efi",
        flat_dir(GeneratePartition::Boot, profile)
    );
    normalize_path(rel_path)
}

pub fn flat_pkg_bin(pkg: &Pkg, profile: BuildProfile) -> PathBuf {
    let rel_path = format!(
        "{flat}/pkg/{pkg}/bin/{bin}",
        flat = flat_dir(GeneratePartition::System, profile),
        pkg = pkg.pkg,
        bin = pkg.bin
    );
    normalize_path(rel_path)
}

#[allow(dead_code)]
pub fn flat_pkg_perm(pkg: &Pkg, profile: BuildProfile) -> PathBuf {
    let rel_path = format!(
        "{flat}/pkg/{pkg}/perms/{bin}.toml",
        flat = flat_dir(GeneratePartition::System, profile),
        pkg = pkg.pkg,
        bin = pkg.bin
    );
    normalize_path(rel_path)
}

pub fn img(partition: GeneratePartition, profile: BuildProfile) -> PathBuf {
    let name = match partition {
        GeneratePartition::Boot => "boot",
        GeneratePartition::System => "system",
        GeneratePartition::Disk => "pentos",
    };

    let rel_path = format!("{}/{}.img", img_dir(profile), name);
    normalize_path(rel_path)
}
