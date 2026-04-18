
pub mod primitive;
pub mod run_policy;

use {crate::target::run_policy::RunPolicy, std::rc::Rc};

pub trait Target {
    fn spec(&self) -> bool;
    fn run_policy(&self) -> Box<dyn RunPolicy>;
    fn dependencies(&self) -> Vec<Rc<dyn Target>>;

    fn run(&self) -> bool {
        let policy = self.run_policy();
        let mut deps_changed = false;

        for dep in self.dependencies() {
            deps_changed |= dep.run();
        }

        if !policy.should_run(deps_changed) {
            return false;
        }

        self.spec()
    }
}
