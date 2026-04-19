use {
    crate::result::ResultExt,
    cargo_metadata::{Metadata, MetadataCommand},
    std::sync::LazyLock,
};

pub static METADATA: LazyLock<Metadata> = LazyLock::new(|| {
    MetadataCommand::new()
        .exec()
        .or_fatal("could not read project metadata")
});
