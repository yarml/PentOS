use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Parser)]
pub struct ChefArgs {
    #[command(subcommand)]
    pub command: ChefCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChefCommand {
    Check,
    Build { package: String },
    Image,
    Run,
    Ovmf,
}
