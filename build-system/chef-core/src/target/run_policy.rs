use {
    crate::result::ResultExt,
    std::{fs, path::PathBuf},
};

pub trait RunPolicy {
    fn should_run(&self, deps: bool) -> bool;
}

pub struct AlwaysRun;
pub struct MirrorDeps;
pub struct FilesNotExist(pub Vec<PathBuf>);

#[allow(dead_code)]
pub struct TimestampsCompare {
    targets: Vec<PathBuf>,
    dependencies: Vec<PathBuf>,
}
pub struct CombinedRunPolicies(pub Vec<Box<dyn RunPolicy>>);

impl FilesNotExist {
    pub fn one_file(target: PathBuf) -> Self {
        Self(vec![target])
    }
}

#[allow(dead_code)]
impl TimestampsCompare {
    pub fn one_target(target: PathBuf, dependencies: Vec<PathBuf>) -> Self {
        Self {
            targets: vec![target],
            dependencies,
        }
    }
    pub fn many_targets(targets: Vec<PathBuf>, dependencies: Vec<PathBuf>) -> Self {
        Self {
            targets,
            dependencies,
        }
    }
}

impl RunPolicy for AlwaysRun {
    fn should_run(&self, _deps: bool) -> bool {
        true
    }
}

impl RunPolicy for MirrorDeps {
    fn should_run(&self, deps: bool) -> bool {
        deps
    }
}

impl RunPolicy for FilesNotExist {
    fn should_run(&self, _deps: bool) -> bool {
        self.0
            .iter()
            .any(|file_path| !fs::exists(file_path).or_fatal("file exist"))
    }
}

impl RunPolicy for TimestampsCompare {
    fn should_run(&self, _deps: bool) -> bool {
        for target in &self.targets {
            if !fs::exists(target).or_fatal("file exist") {
                return true;
            }
            let target_metadata = fs::metadata(target).or_fatal("file metadata");
            let target_time = target_metadata.modified().expect("file metadata modified");
            for dep in &self.dependencies {
                if !fs::exists(dep).or_fatal("file exist") {
                    continue;
                }
                let dep_metadata = fs::metadata(dep).or_fatal("file metadata");
                let dep_time = dep_metadata.modified().or_fatal("file metadata modified");
                if target_time.elapsed().or_fatal("elapsed").as_millis()
                    > dep_time.elapsed().or_fatal("elapsed").as_millis()
                {
                    return true;
                }
            }
        }
        todo!()
    }
}

impl RunPolicy for CombinedRunPolicies {
    fn should_run(&self, deps: bool) -> bool {
        self.0.iter().any(|policy| policy.should_run(deps))
    }
}
