use chef_core::{status::Status, target::Target, targets};

fn main() {
    let target = targets::check::check();
    target.run();
    Status::doing("Done", "");
}
