const POLY: u32 = 0xEDB88320;

const TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ POLY;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

pub fn crc32(data: &[u8]) -> u32 {
    crc32_continue(data, 0)
}

pub fn crc32_continue(data: &[u8], mut crc: u32) -> u32 {
    crc ^= 0xFFFFFFFF;
    for &byte in data {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ TABLE[index];
    }
    crc ^ 0xFFFFFFFF
}

pub fn crc32_zdata(len: usize) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for byte in (0..len).map(|_| 0u8) {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ TABLE[index];
    }
    crc ^ 0xFFFFFFFF
}
