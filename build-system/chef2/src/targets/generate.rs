pub mod img;

use {
    crate::{
        args::{BuildProfile, GeneratePartition},
        crates, paths,
        target::{
            Target,
            primitive::{CopyTarget, SumTarget},
        },
        targets::build,
    },
    std::rc::Rc,
};

pub fn flat(partition: GeneratePartition, profile: BuildProfile) -> SumTarget {
    let mut targets: Vec<Rc<dyn Target>> = vec![];
    if partition.has_main() {
        targets.push(Rc::new(CopyTarget::new(
            Rc::new(build::kernel(profile)),
            paths::target_kernel(profile),
            paths::flat_kernel(profile),
        )));

        for p in crates::all_pkgs() {
            targets.push(Rc::new(CopyTarget::new(
                Rc::new(build::pkg(&p.name, profile)),
                paths::target_pkg(&p.name, profile),
                paths::flat_pkg_bin(p.pkg.as_ref().unwrap(), profile),
            )));
        }
    }
    if partition.has_boot() {
        targets.push(Rc::new(CopyTarget::new(
            Rc::new(build::bootloader(profile)),
            paths::target_bootloader(profile),
            paths::flat_bootloader(profile),
        )));
    }

    SumTarget(
        String::from("Making"),
        format!("flat partition ({})", paths::flat_dir(partition, profile)),
        targets,
    )
}
