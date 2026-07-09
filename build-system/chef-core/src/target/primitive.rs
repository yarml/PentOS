use std::{path::PathBuf, rc::Rc};

use crate::{
    files,
    status::Status,
    target::{
        Target,
        run_policy::{AlwaysRun, CombinedRunPolicies, FilesNotExist, MirrorDeps, RunPolicy},
    },
};

pub struct SumTarget(pub String, pub String, pub Vec<Rc<dyn Target>>);
pub struct CopyTarget {
    src: PathBuf,
    dst: PathBuf,
    make_source: Rc<dyn Target>,
}

impl CopyTarget {
    pub fn new(make_source: Rc<dyn Target>, src: PathBuf, dst: PathBuf) -> Self {
        Self {
            src,
            dst,
            make_source,
        }
    }
}

impl Target for SumTarget {
    fn spec(&self) -> bool {
        Status::push(&self.0, &self.1);
        let mut any_changed = false;

        for subtarget in &self.2 {
            any_changed |= subtarget.run();
        }
        Status::pop();

        any_changed
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![]
    }
}
impl Target for CopyTarget {
    fn spec(&self) -> bool {
        files::copy(&self.src, &self.dst);
        true
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        let policies: Vec<Box<dyn RunPolicy>> = vec![
            Box::new(MirrorDeps),
            Box::new(FilesNotExist::one_file(self.dst.clone())),
        ];

        Box::new(CombinedRunPolicies(policies))
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![self.make_source.clone()]
    }
}
