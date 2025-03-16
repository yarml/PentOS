use cargo_metadata::Package;
use cargo_metadata::camino::Utf8Path;

pub fn get_path(package: &Package) -> &Utf8Path {
    let mut path = package.manifest_path.components();
    path.next_back();
    path.as_path()
}
