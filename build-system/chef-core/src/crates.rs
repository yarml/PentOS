use {
    crate::{metadata::METADATA, paths::normalize_path, result::ResultExt, status::Status},
    cargo_metadata::camino::Utf8Path,
    serde::Deserialize,
    std::{collections::HashMap, fs, path::PathBuf, sync::LazyLock},
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
        let driver = extract_driver_info(Utf8Path::from_path(&path).unwrap());

        let package = Crate {
            name: name.clone(),
            path,
            pkg,
            driver,
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
    pub driver: Option<Driver>,
}

#[derive(Debug, Clone)]
pub struct Pkg {
    pub pkg: String,
    pub bin: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Driver {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
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

pub fn all_drivers() -> impl Iterator<Item = &'static Crate> {
    CRATES.values().filter(|p| p.driver.is_some())
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

fn extract_driver_info(normalized_path: &Utf8Path) -> Option<Driver> {
    let path = normalized_path.to_owned();
    let driver_decl_path = path.join("driver.toml");

    let driver: Driver = toml::from_str(&fs::read_to_string(&driver_decl_path).ok()?)
        .unwrap_or_else(|_| panic!("{driver_decl_path} has wrong format"));

    Some(driver)
}
