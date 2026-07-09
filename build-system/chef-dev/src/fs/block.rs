use {
    block::{BlockDevice, BlockDeviceDimensions},
    chef_core::result::ResultExt,
    io::{IoError, IoResult},
    log::trace,
    std::{
        fs::{self, File},
        io::{Read, Seek, SeekFrom, Write},
        path::Path,
        pin::Pin,
        sync::Mutex,
    },
};

pub struct FileBlockDevice {
    fd: Mutex<File>,
    frame_size: usize,
    page_size: usize,
    page_count: usize,
}

impl FileBlockDevice {
    pub fn create(path: &Path, page_size: usize, page_count: usize, frame_size: usize) -> Self {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).or_fatal("create dir all");
        }
        let mut fd = File::options()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(path)
            .or_fatal("open");

        let total_size = page_count * page_size;

        let m1b_count = total_size / (1024 * 1024);
        let trailing = total_size - m1b_count * 1024 * 1024;
        let m1bz = vec![0u8; 1024 * 1024].into_boxed_slice();

        for _ in 0..m1b_count {
            fd.write_all(&m1bz).or_fatal("write");
        }
        fd.write_all(&m1bz[..trailing]).or_fatal("write");
        fd.seek(SeekFrom::Start(0)).or_fatal("seek");

        Self {
            fd: Mutex::new(fd),
            page_count,
            page_size,
            frame_size,
        }
    }

    pub fn open(path: &Path, page_size: usize, frame_size: usize) -> Self {
        let total_size = fs::metadata(path).or_fatal("metadata").len() as usize;
        let page_count = total_size / page_size;

        let fd = File::options()
            .read(true)
            .write(true)
            .open(path)
            .or_fatal("open");

        Self {
            fd: Mutex::new(fd),
            page_size,
            page_count,
            frame_size,
        }
    }
}

impl FileBlockDevice {
    async fn read_pages_impl(&self, pg: usize, buf: &mut [u8]) -> IoResult<()> {
        assert!(buf.len().is_multiple_of(self.page_size));

        let offset = pg * self.page_size;
        let mut fd = self.fd.lock().or_fatal("lock");
        fd.seek(SeekFrom::Start(offset as u64))
            .map_err(|_| IoError::Unknown)?;
        fd.read_exact(buf).map_err(|_| IoError::Unknown)
    }

    async fn write_pages_impl(&self, pg: usize, buf: &[u8]) -> IoResult<()> {
        assert!(buf.len().is_multiple_of(self.page_size));

        let offset = pg * self.page_size;
        let mut fd = self.fd.lock().or_fatal("lock");
        fd.seek(SeekFrom::Start(offset as u64))
            .map_err(|_| IoError::Unknown)?;

        trace!(
            "writing into pg {pg} => offset {offset} (as page_size {page_size}): {buf:?}",
            page_size = self.page_size
        );

        fd.write_all(buf).map_err(|_| IoError::Unknown)
    }

    async fn flush_impl(&self) -> IoResult<()> {
        let mut fd = self.fd.lock().or_fatal("lock");
        fd.flush().map_err(|_| IoError::Unknown)
    }
}

impl BlockDevice for FileBlockDevice {
    fn dimensions(&self) -> BlockDeviceDimensions {
        BlockDeviceDimensions {
            page_count: self.page_count,
            page_size: self.page_size,
            frame_size: Some(self.frame_size),
            optimal_transfer_size: Some(1024 * 1024),
        }
    }

    fn read_pages<'a>(
        &'a self,
        pg: usize,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.read_pages_impl(pg, buf))
    }

    fn write_pages<'a>(
        &'a self,
        pg: usize,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.write_pages_impl(pg, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.flush_impl())
    }
}
