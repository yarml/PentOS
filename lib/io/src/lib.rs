#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoError {
    Unknown,
    InUse,
    Eof,
    InvalidInput,
    OutOfBounds,
    NotFound,
    AlreadyExists,
    NoSpace,
    Unsupported,
    Corrupted,
}

pub type IoResult<T> = Result<T, IoError>;

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize>;

    fn read_exact(&mut self, mut buf: &mut [u8]) -> IoResult<()> {
        while !buf.is_empty() {
            let amount = self.read(buf)?;
            buf = &mut buf[amount..];
        }
        IoResult::Ok(())
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize>;
    fn flush(&mut self) -> IoResult<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> IoResult<()> {
        while !buf.is_empty() {
            let amount = self.write(buf)?;
            buf = &buf[amount..];
        }
        IoResult::Ok(())
    }
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64>;

    fn stream_position(&mut self) -> IoResult<u64> {
        self.seek(SeekFrom::Current(0))
    }

    fn stream_len(&mut self) -> IoResult<u64> {
        let current = self.stream_position()?;
        let end = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(current))?;
        IoResult::Ok(end - current)
    }
}
