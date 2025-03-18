use std::process::Command;

pub struct Cargo {
    subcommand: String,
    packages: Packages,
    targets: Vec<String>,
    keep_going: bool,
    quiet: bool,
    message_format: Option<String>,
    config: Option<String>,
}

pub enum Packages {
    Workspace { exclude: Vec<String> },
    PackageList(Vec<String>),
    Unspecified,
}

impl Cargo {
    pub fn check() -> Cargo {
        Cargo {
            subcommand: String::from("check"),
            packages: Packages::Unspecified,
            targets: vec![],
            keep_going: false,
            quiet: false,
            message_format: None,
            config: None,
        }
    }
    pub fn build() -> Cargo {
        Cargo {
            subcommand: String::from("build"),
            packages: Packages::Unspecified,
            targets: vec![],
            keep_going: false,
            quiet: false,
            message_format: None,
            config: None,
        }
    }
    pub fn command(self) -> Command {
        let mut cmd = Command::new("cargo");
        self.append_args(&mut cmd);
        cmd
    }
}

impl Cargo {
    pub fn packages(mut self, packages: Packages) -> Self {
        self.packages = packages;
        self
    }
    pub fn with_target(mut self, targets: String) -> Self {
        self.targets.push(targets);
        self
    }
    pub fn keep_going(mut self) -> Self {
        self.keep_going = true;
        self
    }
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }
    pub fn message_format(mut self, message_format: &str) -> Self {
        self.message_format = Some(message_format.to_string());
        self
    }
    pub fn config(mut self, config: String) -> Self {
        self.config = Some(config);
        self
    }
}

impl Packages {
    pub fn workspace() -> Packages {
        Packages::Workspace { exclude: vec![] }
    }
    pub fn workspace_except(exclude: Vec<String>) -> Packages {
        Packages::Workspace { exclude }
    }
    pub fn package_list(items: &[&str]) -> Packages {
        Packages::PackageList(items.iter().map(|item| item.to_string()).collect())
    }
}

impl Cargo {
    fn append_args(&self, cmd: &mut Command) {
        cmd.arg(&self.subcommand);
        self.packages.append_args(cmd);
        for target in &self.targets {
            cmd.arg("--target").arg(target);
        }
        if self.keep_going {
            cmd.arg("--keep-going");
        }
        if self.quiet {
            cmd.arg("--quiet");
        }
        if let Some(message_format) = &self.message_format {
            cmd.arg("--message-format").arg(message_format);
        }
        if let Some(config) = &self.config {
            cmd.arg("--config").arg(config);
        }
    }
}

impl Packages {
    fn append_args(&self, cmd: &mut Command) {
        match self {
            Packages::Workspace { exclude } => {
                cmd.arg("--workspace");
                for item in exclude {
                    cmd.arg("--exclude").arg(item);
                }
            }
            Packages::PackageList(items) => {
                for item in items {
                    cmd.arg("--package").arg(item);
                }
            }
            Packages::Unspecified => {}
        }
    }
}
