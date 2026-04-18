use {
    crate::file::File,
    alloc::{boxed::Box, string::String, sync::Arc, vec::Vec},
    core::pin::Pin,
    io::IoResult,
};

pub type DirFuture<'a, T> = Pin<Box<dyn Future<Output = IoResult<T>> + Send + 'a>>;

pub trait Filesystem: Send + Sync {
    fn root(&self) -> Arc<dyn Directory>;
}

/// A directory.
///
/// All methods take `&self`. Two `Arc<dyn Directory>`s pointing at the same
/// directory may be used concurrently from different tasks; the
/// implementation is responsible for whatever interior locking is needed.
pub trait Directory: Send + Sync {
    fn create_file<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<File>>;
    fn create_dir<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<dyn Directory>>;
    fn open_file<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<File>>;
    fn open_dir<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<dyn Directory>>;
    fn list<'a>(&'a self) -> DirFuture<'a, Vec<DirEntry>>;
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
}
