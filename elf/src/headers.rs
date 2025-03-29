use crate::types::Addr;
use crate::types::Half;
use crate::types::Offset;
use crate::types::UChar;
use crate::types::Word;
use crate::types::XWord;
use core::mem;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FileHeader {
    pub ident: [UChar; 16],
    pub ty: Half,
    pub machine: Half,
    pub version: Word,
    pub entry: Addr,
    pub phoff: Offset,
    pub shoff: Offset,
    pub flags: Word,
    pub ehsize: Half,
    pub phentsize: Half,
    pub phnum: Half,
    pub shentsize: Half,
    pub shnum: Half,
    pub shstrndx: Half,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawSegment {
    pub ty: Word,
    pub flags: Word,
    pub offset: Offset,
    pub vaddr: Addr,
    pub paddr: Addr,
    pub file_size: XWord,
    pub mem_size: XWord,
    pub alignment: XWord,
}

impl FileHeader {
    pub fn from(data: &[u8]) -> Option<&FileHeader> {
        const HEADER_SIZE: usize = mem::size_of::<FileHeader>();
        const HEADER_ALIGN: usize = mem::align_of::<FileHeader>();
        if data.len() < HEADER_SIZE || !data.as_ptr().is_aligned_to(HEADER_ALIGN) {
            return None;
        }

        Some(unsafe {
            // SAFETY: `data` is aligned to `HEADER_ALIGN` and has at least length `HEADER_SIZE`. All states are valid.
            &*(data.as_ptr() as *const _)
        })
    }
}

impl RawSegment {
    pub fn from(data: &[u8]) -> Option<&RawSegment> {
        const SEGMENT_SIZE: usize = mem::size_of::<RawSegment>();
        const SEGMENT_ALIGN: usize = mem::align_of::<RawSegment>();
        if data.len() < SEGMENT_SIZE || !data.as_ptr().is_aligned_to(SEGMENT_ALIGN) {
            return None;
        }

        Some(unsafe {
            // SAFETY: `data` is aligned to `SEGMENT_ALIGN` and has at least length `SEGMENT_SIZE`. All states are valid.
            &*(data.as_ptr() as *const _)
        })
    }
}
