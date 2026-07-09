use {
    crate::{
        command::{self, CommandOptions},
        crates::{self, Crate},
        target::{
            Target,
            run_policy::{AlwaysRun, RunPolicy},
        },
    },
    std::{process::Command, rc::Rc},
};

pub fn check() -> CheckTarget {
    CheckTarget { json: true }
}

pub fn lint() -> CheckTarget {
    CheckTarget { json: false }
}

pub struct CheckTarget {
    json: bool,
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
