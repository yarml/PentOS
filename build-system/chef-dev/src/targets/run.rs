use {
    crate::{args::BuildProfile, qemu, targets},
    chef_core::{
        command,
        target::{
            Target,
            run_policy::{AlwaysRun, RunPolicy},
        },
    },
    std::rc::Rc,
};

pub fn run(profile: BuildProfile) -> RunTarget {
    RunTarget {
        profile,
        debug: false,
    }
}

pub fn debug() -> RunTarget {
    RunTarget {
        profile: BuildProfile::Debug,
        debug: true,
    }
}

pub struct RunTarget {
    profile: BuildProfile,
    debug: bool,
}

impl Target for RunTarget {
    fn spec(&self) -> bool {
        if !self.debug {
            command::exec(qemu::base(self.profile));
        } else {
            command::exec(qemu::debug());
        }
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![
            Rc::new(targets::generate::img::disk(self.profile, 512, 512)),
            Rc::new(targets::download::ovmf()),
        ]
    }
}
