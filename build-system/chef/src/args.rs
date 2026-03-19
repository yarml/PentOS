use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct ChefArgs {
    #[command(subcommand)]
    pub command: ChefCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChefCommand {
    Ovmf,
    Packages {
        #[command(subcommand)]
        command: PackagesCommand,
    },
    Config {
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum PackagesCommand {
    Name,
    Path,
    Userbin,
}
