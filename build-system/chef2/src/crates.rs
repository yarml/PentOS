use {
    crate::{metadata::METADATA, paths::normalize_path, result::ResultExt, status::Status},
    cargo_metadata::camino::Utf8Path,
    std::{collections::HashMap, path::PathBuf, sync::LazyLock},
};

static CRATES: LazyLock<HashMap<String, Crate>> = LazyLock::new(|| {
    let mut packages = HashMap::new();

    for pkg_id in &METADATA.workspace_members {
        let pkg = &METADATA[pkg_id];
        let name = pkg.name.to_string();
        let mut path = pkg.manifest_path.clone();
        path.pop();
        let path = normalize_path(path);
        let pkg = extract_pkg_info(Utf8Path::from_path(&path).unwrap());

        let package = Crate {
            name: name.clone(),
            path,
            pkg,
        };

        if packages.insert(name.clone(), package).is_some() {
            Status::error(format!("package {name} defined multiple times"));
        }
    }

    packages
});

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub path: PathBuf,
    pub pkg: Option<Pkg>,
}

#[derive(Debug, Clone)]
pub struct Pkg {
    pub pkg: String,
    pub bin: String,
}

pub fn find_crate(name: &str) -> &'static Crate {
    CRATES
        .get(name)
        .ok_or("package not found")
        .or_fatal("find package")
}

pub fn all_crates() -> impl Iterator<Item = &'static Crate> {
    CRATES.values()
}

pub fn all_pkgs() -> impl Iterator<Item = &'static Crate> {
    CRATES.values().filter(|p| p.pkg.is_some())
}

fn extract_pkg_info(normalized_path: &Utf8Path) -> Option<Pkg> {
    let mut path = normalized_path.to_owned();
    let bin = String::from(path.file_name()?);
    path.pop();
    let pkg = String::from(path.file_name()?);
    path.pop();
    let userdir = String::from(path.file_name()?);
    if pkg == "lib" {
        return None;
    }
    if userdir != "user" {
        return None;
    }

    Some(Pkg { pkg, bin })
}
