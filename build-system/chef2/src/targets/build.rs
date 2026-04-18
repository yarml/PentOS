use {
    crate::{
        args::BuildProfile,
        command,
        crates::{Crate, find_crate},
        paths,
        result::ResultExt,
        target::{
            Target,
            run_policy::{AlwaysRun, RunPolicy},
        },
    },
    std::{fs, path::PathBuf, process::Command, rc::Rc},
};

pub fn bootloader(profile: BuildProfile) -> BuildTarget {
    BuildTarget::new("bootloader", paths::target_bootloader(profile), profile)
}
pub fn kernel(profile: BuildProfile) -> BuildTarget {
    BuildTarget::new("kernel", paths::target_kernel(profile), profile)
}
pub fn pkg(name: &str, profile: BuildProfile) -> BuildTarget {
    BuildTarget::new(name, paths::target_pkg(name, profile), profile)
}

pub struct BuildTarget {
    profile: BuildProfile,
    package: &'static Crate,
    output_bin: PathBuf,
}

impl BuildTarget {
    pub fn new(pkg_name: &str, output_bin: PathBuf, profile: BuildProfile) -> Self {
        Self {
            profile,
            package: find_crate(pkg_name),
            output_bin,
        }
    }
}

impl Target for BuildTarget {
    fn spec(&self) -> bool {
        let c0 = fs::read(&self.output_bin)
            .ok()
            .map(|data| md5::compute(&data));

        let mut command = Command::new("cargo");
        command.current_dir(&self.package.path);
        command.arg("build");
        command.arg("-p").arg(&self.package.name);
        if self.profile == BuildProfile::Release {
            command.arg("--release");
        }
        command::exec(command, true);

        if let Some(c0) = c0 {
            let c1 = md5::compute(fs::read(&self.output_bin).or_fatal("read"));
            c0 != c1
        } else {
            true
        }
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        // FIXME: kernel depends on font, we could have user programs also depend on font
        vec![]
    }
}
