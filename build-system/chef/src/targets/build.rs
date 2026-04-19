use {
    crate::{
        args::BuildProfile,
        command::{self, CommandOptions},
        crates::{self, Crate, find_crate},
        paths,
        result::ResultExt,
        status::Status,
        target::{
            Target,
            run_policy::{AlwaysRun, RunPolicy},
        },
        targets,
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

pub fn check() -> CheckTarget {
    CheckTarget { json: true }
}

pub fn lint() -> CheckTarget {
    CheckTarget { json: false }
}

pub fn doc() -> DocTarget {
    DocTarget
}

pub fn test() -> TestTarget {
    TestTarget
}

pub struct BuildTarget {
    profile: BuildProfile,
    package: &'static Crate,
    output_bin: PathBuf,
}

pub struct CheckTarget {
    json: bool,
}
pub struct TestTarget;
pub struct DocTarget;

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
        command::run(command, CommandOptions::new());

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
        vec![Rc::new(targets::download::font())]
    }
}

impl Target for CheckTarget {
    fn spec(&self) -> bool {
        for p in crates::all_crates() {
            let mut command = base_lint(p);
            if self.json {
                command.arg("--message-format=json");

            }
            command::run(command, CommandOptions::new());
        }
        false
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![]
    }
}

impl Target for TestTarget {
    fn spec(&self) -> bool {
        let mut command = Command::new("cargo");
        command.args(["test", "-p", "test"]);
        command::exec(command);
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![]
    }
}

impl Target for DocTarget {
    fn spec(&self) -> bool {
        let mut command = Command::new("cargo");
        command.args(["doc", "--workspace", "--release", "--no-deps"]);
        command::run(command, CommandOptions::new());

        Status::doing("Writing", "index.html");
        std::fs::write(
            "target/doc/index.html",
            r#"<meta http-equiv="refresh" content="0; url=pentos/">"#,
        )
        .or_fatal("write index.html");

        false
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![]
    }
}

fn base_lint(p: &Crate) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(&p.path)
        .arg("clippy")
        .arg("--all-features")
        .arg("--no-deps")
        .arg("--keep-going")
        .arg("--quiet")
        .args(["-p", &p.name]);

    command
}
