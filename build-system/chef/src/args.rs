use {
    crate::{target::Target, targets},
    clap::{Parser, Subcommand, ValueEnum},
    std::rc::Rc,
};

#[derive(Debug, Parser)]
#[command(version, author, about)]
pub struct ChefCli {
    #[command(subcommand)]
    pub command: ChefCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChefCommand {
    Download {
        #[arg(value_enum)]
        asset: DownloadAsset,
    },
    Build {
        #[command(subcommand)]
        component: BuildComponent,
        #[arg(
            value_enum,
            short = 'p', long = "profile",
            default_value_t = BuildProfile::Release
        )]
        profile: BuildProfile,
    },
    Generate {
        #[arg(value_enum)]
        format: GenerateFormat,
        #[arg(value_enum, default_value_t = GeneratePartition::Disk)]
        partition: GeneratePartition,
        #[arg(
            value_enum,
            short = 'p', long = "profile",
            default_value_t = BuildProfile::Release
        )]
        profile: BuildProfile,
        #[arg(short = 's', long = "page-size", default_value_t = 512)]
        page_size: usize,
        #[arg(short = 'f', long = "frame-size", default_value_t = 4096)]
        frame_size: usize,
    },

    Run {
        #[arg(
            value_enum,
            short = 'p', long = "profile",
            default_value_t = BuildProfile::Release
        )]
        profile: BuildProfile,
    },
    Debug,

    Check,
    Lint,
    Doc,
    Test,

    Info {
        #[arg(value_enum)]
        info: InfoCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DownloadAsset {
    Ovmf,
    Font,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BuildProfile {
    Debug,
    Release,
}

#[derive(Debug, Subcommand)]
pub enum BuildComponent {
    Kernel,
    Bootloader,
    Pkg { name: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GenerateFormat {
    Flat,
    Img,
    // TODO: Iso,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GeneratePartition {
    Boot,
    System,
    Disk,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InfoCommand {
    Lines,
}

impl ChefCommand {
    pub fn get_target(&self) -> Rc<dyn Target> {
        match self {
            ChefCommand::Download {
                asset: DownloadAsset::Ovmf,
            } => Rc::new(targets::download::ovmf()),
            ChefCommand::Download {
                asset: DownloadAsset::Font,
            } => Rc::new(targets::download::font()),
            ChefCommand::Build {
                component: BuildComponent::Bootloader,
                profile,
            } => Rc::new(targets::build::bootloader(*profile)),
            ChefCommand::Build {
                component: BuildComponent::Kernel,
                profile,
            } => Rc::new(targets::build::kernel(*profile)),
            ChefCommand::Build {
                component: BuildComponent::Pkg { name },
                profile,
            } => Rc::new(targets::build::pkg(name, *profile)),
            ChefCommand::Generate {
                format: GenerateFormat::Flat,
                partition,
                profile,
                ..
            } => Rc::new(targets::generate::flat(*partition, *profile)),
            ChefCommand::Generate {
                format: GenerateFormat::Img,
                partition: GeneratePartition::Disk,
                profile,
                page_size,
                frame_size,
            } => Rc::new(targets::generate::img::disk(
                *profile,
                *page_size,
                *frame_size,
            )),
            ChefCommand::Generate {
                format: GenerateFormat::Img,
                partition: GeneratePartition::Boot,
                profile,
                page_size,
                frame_size,
            } => Rc::new(targets::generate::img::boot(
                *profile,
                *page_size,
                *frame_size,
            )),
            ChefCommand::Generate {
                format: GenerateFormat::Img,
                partition: GeneratePartition::System,
                profile,
                page_size,
                frame_size,
            } => Rc::new(targets::generate::img::system(
                *profile,
                *page_size,
                *frame_size,
            )),
            ChefCommand::Run { profile } => Rc::new(targets::run::run(*profile)),
            ChefCommand::Debug => Rc::new(targets::run::debug()),
            ChefCommand::Check => Rc::new(targets::build::check()),
            ChefCommand::Lint => Rc::new(targets::build::lint()),
            ChefCommand::Doc => Rc::new(targets::build::doc()),
            ChefCommand::Test => Rc::new(targets::build::test()),
            ChefCommand::Info {
                info: InfoCommand::Lines,
            } => Rc::new(targets::info::lines()),
        }
    }
}

impl BuildProfile {
    pub fn pathname(&self) -> &'static str {
        match self {
            BuildProfile::Debug => "debug",
            BuildProfile::Release => "release",
        }
    }
}

impl GeneratePartition {
    pub fn has_main(&self) -> bool {
        matches!(self, Self::Disk | Self::System)
    }
    pub fn has_boot(&self) -> bool {
        matches!(self, Self::Disk | Self::Boot)
    }
}
