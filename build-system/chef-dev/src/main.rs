mod args;
mod fs;
mod paths;
mod qemu;
mod targets;

use {crate::args::ChefCli, chef_core::status::Status, clap::Parser, log::info};

fn main() {
    env_logger::init();
    info!("chef startup");
    let cli = ChefCli::parse();
    let target = cli.command.get_target();

    target.run();
    Status::doing("Done", "");
}
