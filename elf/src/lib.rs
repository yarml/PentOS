#![no_std]
#![feature(pointer_is_aligned_to)]

pub mod headers;
pub mod types;

use common::mem::MemorySize;
use common::mem::addr::VirtAddr;
use core::ops::Index;
use headers::FileHeader;
use headers::RawSegment;
use types::Half;
use types::Offset;
use types::UChar;
use types::Word;

pub struct Elf<'a> {
    pub ident: ElfIdentification,
    pub ty: ElfType,
    pub entry: VirtAddr,
    pub program_header: ProgramHeader<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfIdentification {
    pub class: ElfClass,
    pub encoding: DataEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf32,
    Elf64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataEncoding {
    LittleEndian,
    BigEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfType {
    Relocatable,
    Executable,
    SharedObject,
    Core,
}

#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub ty: SegmentType,
    pub flags: SegmentFlags,
    pub offset: Offset,
    pub vaddr: VirtAddr,
    pub file_size: usize,
    pub mem_size: MemorySize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentType {
    Null,
    Load,
    Dynamic,
    Interpreter,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentFlags {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

pub struct ProgramHeader<'a> {
    raw: &'a [u8],
    entry_size: usize,
    pub len: usize,
}

pub struct SegmentIter<'a, 'b> {
    program_header: &'b ProgramHeader<'a>,
    index: usize,
}

impl<'a> Elf<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let file_header = FileHeader::from(data)?;

        let ident = ElfIdentification::parse(&file_header.ident)?;
        let ty = ElfType::parse(file_header.ty)?;
        let entry = VirtAddr::new(file_header.entry as usize)?;
        let program_header = ProgramHeader::new(
            &data[file_header.phoff as usize..],
            file_header.phentsize as usize,
            file_header.phnum as usize,
        );

        Some(Self {
            ident,
            ty,
            entry,
            program_header,
        })
    }
}

impl ElfIdentification {
    pub fn parse(data: &[UChar; 16]) -> Option<Self> {
        if &data[0..4] != b"\x7FELF" || data[6] != 1 || data[7] != 0 {
            return None;
        }
        let class = ElfClass::parse(data[4])?;
        let encoding = DataEncoding::parse(data[5])?;
        Some(Self { class, encoding })
    }
}

impl ElfClass {
    pub fn parse(class: UChar) -> Option<Self> {
        match class {
            1 => Some(Self::Elf32),
            2 => Some(Self::Elf64),
            _ => None,
        }
    }
}

impl DataEncoding {
    pub fn parse(encoding: UChar) -> Option<Self> {
        match encoding {
            1 => Some(Self::LittleEndian),
            2 => Some(Self::BigEndian),
            _ => None,
        }
    }
}

impl ElfType {
    pub fn parse(ty: Half) -> Option<Self> {
        match ty {
            1 => Some(Self::Relocatable),
            2 => Some(Self::Executable),
            3 => Some(Self::SharedObject),
            4 => Some(Self::Core),
            _ => None,
        }
    }
}

impl Segment {
    pub fn parse(raw: &RawSegment) -> Option<Self> {
        let ty = SegmentType::parse(raw.ty)?;
        let flags = SegmentFlags::parse(raw.flags);
        let offset = raw.offset;
        let vaddr = VirtAddr::new(raw.vaddr as usize)?;
        let file_size = raw.file_size as usize;
        let mem_size = MemorySize::new(raw.mem_size as usize);
        let alignment = raw.alignment as usize;
        if *vaddr % alignment != 0 {
            return None;
        }
        Some(Self {
            ty,
            flags,
            offset,
            vaddr,
            file_size,
            mem_size,
        })
    }
}

impl SegmentType {
    pub fn parse(ty: Word) -> Option<Self> {
        match ty {
            0 => Some(Self::Null),
            1 => Some(Self::Load),
            2 => Some(Self::Dynamic),
            3 => Some(Self::Interpreter),
            4 => Some(Self::Note),
            _ => None,
        }
    }
}

impl SegmentFlags {
    pub fn parse(flags: Word) -> Self {
        Self {
            read: flags & 1 != 0,
            write: flags & 2 != 0,
            exec: flags & 4 != 0,
        }
    }
}

impl<'a> ProgramHeader<'a> {
    pub fn new(raw: &'a [u8], entry_size: usize, entry_count: usize) -> Self {
        Self {
            raw,
            entry_size,
            len: entry_count,
        }
    }
}

impl<'a> Index<usize> for ProgramHeader<'a> {
    type Output = RawSegment;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len);
        let offset = index * self.entry_size;
        RawSegment::from(&self.raw[offset..offset + self.entry_size]).expect("Invalid segment")
    }
}

impl<'a, 'b> IntoIterator for &'b ProgramHeader<'a> {
    type Item = Segment;
    type IntoIter = SegmentIter<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        SegmentIter::<'a, 'b> {
            program_header: self,
            index: 0,
        }
    }
}

impl<'a, 'b> Iterator for SegmentIter<'a, 'b> {
    type Item = Segment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.program_header.len {
            let segment = Segment::parse(&self.program_header[self.index]);
            self.index += 1;
            segment
        } else {
            None
        }
    }
}
