mod args;
mod command;
mod config;
mod crates;
mod files;
mod fs;
mod metadata;
mod paths;
mod result;
mod status;
mod target;
mod targets;
mod task;
mod qemu;

use {
    crate::{args::ChefCli, status::Status},
    clap::Parser,
    log::info,
};

fn main() {
    env_logger::init();
    info!("chef startup");
    let cli = ChefCli::parse();
    let target = cli.command.get_target();

    target.run();
    Status::doing("Done", "");
}
