use {
    alloc::{string::String, vec::Vec},
    core::mem::size_of,
};

pub const DIRENT_SIZE: usize = 32;

// see FAT spec, "FAT 32 Byte Directory Entry Structure"
pub const ATTR_READ_ONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_VOLUME_ID: u8 = 0x08;
pub const ATTR_DIRECTORY: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;

pub const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;
pub const ATTR_LONG_NAME_MASK: u8 =
    ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID | ATTR_DIRECTORY | ATTR_ARCHIVE;

pub const NAME0_FREE: u8 = 0xE5;
pub const NAME0_END: u8 = 0x00;
pub const NAME0_KANJI_E5: u8 = 0x05;

pub const LAST_LONG_ENTRY: u8 = 0x40;

pub const LFN_CHARS_PER_ENTRY: usize = 13;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ShortDirEntry {
    pub name: [u8; 11],
    pub attributes: u8,
    pub nt_res: u8,
    pub creat_time_cs: u8,
    pub creat_time: u16,
    pub creat_date: u16,
    pub last_acc_date: u16,
    pub cluster_hi: u16,
    pub write_time: u16,
    pub write_date: u16,
    pub cluster_lo: u16,
    pub file_size: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct LongFileNameEntry {
    pub order: u8,
    pub name0: [u16; 5],
    pub attributes: u8,
    pub typ: u8,
    pub checksum: u8,
    pub name1: [u16; 6],
    pub cluster_lo: u16,
    pub name2: [u16; 2],
}

const _: () = assert!(size_of::<ShortDirEntry>() == DIRENT_SIZE);
const _: () = assert!(size_of::<LongFileNameEntry>() == DIRENT_SIZE);

impl ShortDirEntry {
    pub const fn cluster(&self) -> usize {
        self.cluster_lo as usize | ((self.cluster_hi as usize) << 16)
    }

    pub const fn set_cluster(&mut self, c: usize) {
        self.cluster_lo = (c & 0xFFFF) as u16;
        self.cluster_hi = ((c >> 16) & 0xFFFF) as u16;
    }

    pub const fn is_dir(&self) -> bool {
        (self.attributes & ATTR_DIRECTORY) != 0
    }

    pub const fn is_volume_id(&self) -> bool {
        (self.attributes & ATTR_LONG_NAME_MASK) == ATTR_VOLUME_ID
    }

    pub fn from_bytes(b: &[u8; DIRENT_SIZE]) -> Self {
        unsafe { core::ptr::read_unaligned(b.as_ptr() as *const Self) }
    }

    pub fn write_to(&self, b: &mut [u8; DIRENT_SIZE]) {
        unsafe { core::ptr::write_unaligned(b.as_mut_ptr() as *mut Self, *self) }
    }
}

impl LongFileNameEntry {
    pub fn from_bytes(b: &[u8; DIRENT_SIZE]) -> Self {
        unsafe { core::ptr::read_unaligned(b.as_ptr() as *const Self) }
    }
    pub fn write_to(&self, b: &mut [u8; DIRENT_SIZE]) {
        unsafe { core::ptr::write_unaligned(b.as_mut_ptr() as *mut Self, *self) }
    }

    pub fn chars(&self) -> [u16; LFN_CHARS_PER_ENTRY] {
        let name0: [u16; 5] = unsafe { core::ptr::read_unaligned(&raw const self.name0) };
        let name1: [u16; 6] = unsafe { core::ptr::read_unaligned(&raw const self.name1) };
        let name2: [u16; 2] = unsafe { core::ptr::read_unaligned(&raw const self.name2) };

        let mut out = [0u16; LFN_CHARS_PER_ENTRY];
        out[0..5].copy_from_slice(&name0);
        out[5..11].copy_from_slice(&name1);
        out[11..13].copy_from_slice(&name2);
        out
    }

    pub fn set_chars(&mut self, chars: &[u16; LFN_CHARS_PER_ENTRY]) {
        let mut name0 = [0u16; 5];
        let mut name1 = [0u16; 6];
        let mut name2 = [0u16; 2];
        name0.copy_from_slice(&chars[0..5]);
        name1.copy_from_slice(&chars[5..11]);
        name2.copy_from_slice(&chars[11..13]);
        unsafe {
            core::ptr::write_unaligned(&raw mut self.name0, name0);
            core::ptr::write_unaligned(&raw mut self.name1, name1);
            core::ptr::write_unaligned(&raw mut self.name2, name2);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    Free,
    End,
    Lfn,
    Sfn,
}

pub fn classify(slot: &[u8; DIRENT_SIZE]) -> SlotKind {
    match slot[0] {
        NAME0_END => SlotKind::End,
        NAME0_FREE => SlotKind::Free,
        _ => {
            let attr = slot[11];
            if (attr & ATTR_LONG_NAME_MASK) == ATTR_LONG_NAME {
                SlotKind::Lfn
            } else {
                SlotKind::Sfn
            }
        }
    }
}

pub fn sfn_checksum(name: &[u8; 11]) -> u8 {
    let mut sum: u8 = 0;
    for &b in name {
        // exactly the algorithm in the spec.
        sum = (if sum & 1 != 0 { 0x80u8 } else { 0u8 })
            .wrapping_add(sum >> 1)
            .wrapping_add(b);
    }
    sum
}

pub fn decode_sfn(raw: &[u8; 11]) -> String {
    let mut out = String::new();

    let mut name = *raw;

    // Undo the Kanji escape on byte 0.
    // Astaghfirullah from the backward compatibility dial had zmer
    // why am i bothering with this 😭😭😭😭😭
    if name[0] == NAME0_KANJI_E5 {
        name[0] = 0xE5;
    }

    let main = trim_trailing_spaces(&name[..8]);
    let ext = trim_trailing_spaces(&name[8..11]);
    push_oem_bytes(&mut out, main);

    if !ext.is_empty() {
        out.push('.');
        push_oem_bytes(&mut out, ext);
    }

    out
}

fn trim_trailing_spaces(b: &[u8]) -> &[u8] {
    let mut end = b.len();
    while end > 0 && b[end - 1] == b' ' {
        end -= 1;
    }
    &b[..end]
}

fn push_oem_bytes(out: &mut String, bytes: &[u8]) {
    for &b in bytes {
        if b < 0x80 {
            out.push(b as char);
        } else {
            out.push('_');
        }
    }
}

pub fn decode_lfn_set(entries: &[LongFileNameEntry]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(entries.len() * LFN_CHARS_PER_ENTRY);
    for e in entries.iter().rev() {
        for c in e.chars() {
            if c == 0x0000 || c == 0xFFFF {
                return String::from_utf16_lossy(&units);
            }
            units.push(c);
        }
    }
    String::from_utf16_lossy(&units)
}

/// Generate a basis 8.3 SFN from a long name, before collision-suffix
/// numbering.
///
/// This is a simplified version of the spec algorithm
///
/// Returns `(name, lossy)`: Callers should append a number whenever
/// `lossy` is true even if there is no collision yet, as our spec overlords have decided.
pub fn basis_name(long: &str) -> ([u8; 11], bool) {
    let mut out = [b' '; 11];
    let mut lossy = false;

    let trimmed = long.trim_start_matches([' ', '.']);
    if trimmed.len() != long.len() {
        lossy = true;
    }

    let (primary, ext) = match trimmed.rfind('.') {
        Some(idx) => (&trimmed[..idx], &trimmed[idx + 1..]),
        None => (trimmed, ""),
    };

    let mut written = 0usize;
    for c in primary.chars() {
        if c == ' ' {
            lossy = true;
            continue;
        }
        if written >= 8 {
            lossy = true;
            break;
        }
        out[written] = sfn_char(c, &mut lossy);
        written += 1;
    }

    for (i, c) in ext.chars().enumerate() {
        if i >= 3 {
            lossy = true;
            break;
        }
        out[8 + i] = sfn_char(c, &mut lossy);
    }

    if out[0] == 0xE5 {
        out[0] = NAME0_KANJI_E5;
    }

    (out, lossy)
}

fn sfn_char(c: char, lossy: &mut bool) -> u8 {
    let cu = c.to_ascii_uppercase();
    let b = cu as u32;
    if b > 0x7F {
        *lossy = true;
        return b'_';
    }
    let b = b as u8;
    let ok = b.is_ascii_uppercase()
        || b.is_ascii_digit()
        || matches!(
            b,
            b'$' | b'%'
                | b'\''
                | b'-'
                | b'_'
                | b'@'
                | b'~'
                | b'`'
                | b'!'
                | b'('
                | b')'
                | b'{'
                | b'}'
                | b'^'
                | b'#'
                | b'&'
        );
    if ok {
        b
    } else {
        *lossy = true;
        b'_'
    }
}

/// Apply a collision suffix to a basis name.
pub fn apply_numeric_tail(basis: &mut [u8; 11], n: u32) {
    let mut tail = [0u8; 8];
    let mut tail_len = 1usize;
    tail[0] = b'~';

    {
        let mut buf = [0u8; 7];
        let mut len = 0usize;
        let mut x = n.max(1);
        while x > 0 && len < buf.len() {
            buf[len] = b'0' + (x % 10) as u8;
            x /= 10;
            len += 1;
        }
        for i in 0..len {
            tail[1 + i] = buf[len - 1 - i];
        }
        tail_len += len;
    }

    let mut primary_end = 8;
    while primary_end > 0 && basis[primary_end - 1] == b' ' {
        primary_end -= 1;
    }
    let insert_at = primary_end.min(8 - tail_len);
    basis[insert_at..(insert_at + tail_len)].copy_from_slice(&tail[..tail_len]);

    // Pad anything after the tail in the primary with spaces.
    for b in basis.iter_mut().take(8).skip(insert_at + tail_len) {
        *b = b' ';
    }
}

pub fn build_lfn_entries(name: &str, checksum: u8) -> Vec<LongFileNameEntry> {
    let units: Vec<u16> = name.encode_utf16().collect();

    let n = units.len().div_ceil(LFN_CHARS_PER_ENTRY).max(1);
    let mut out: Vec<LongFileNameEntry> = Vec::with_capacity(n);

    for ord in (1..=n).rev() {
        let i = ord - 1;

        let start = i * LFN_CHARS_PER_ENTRY;

        let mut chars = [0xFFFFu16; LFN_CHARS_PER_ENTRY];

        for (j, item) in chars.iter_mut().enumerate().take(LFN_CHARS_PER_ENTRY) {
            let k = start + j;
            if k < units.len() {
                *item = units[k];
            } else if k == units.len() {
                *item = 0x0000;
            } else {
                *item = 0xFFFF;
            }
        }

        let mut e = LongFileNameEntry {
            order: ord as u8,
            name0: [0; 5],
            attributes: ATTR_LONG_NAME,
            typ: 0,
            checksum,
            name1: [0; 6],
            cluster_lo: 0,
            name2: [0; 2],
        };

        if ord == n {
            e.order |= LAST_LONG_ENTRY;
        }

        e.set_chars(&chars);
        out.push(e);
    }

    out
}
