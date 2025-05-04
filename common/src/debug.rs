use log::debug;

/// Dump memory with 32 byte per line
pub fn memory_dump(start: usize, size: usize) {
    const BYTE_PER_LINE: usize = 16;
    let aligned_start = start / BYTE_PER_LINE * BYTE_PER_LINE;
    let end = start + size;
    let aligned_end = end.next_multiple_of(BYTE_PER_LINE);
    let start = aligned_start;
    let end = aligned_end;
    let size = end - start;

    debug!(
        "Memory dump {:016x}:{:016x} {:#016x} bytes",
        start, end, size
    );
    for line_start in (start..end).step_by(BYTE_PER_LINE) {
        let mut line = [0u8; BYTE_PER_LINE * 2 + BYTE_PER_LINE];
        let mut head = 0;
        for byte in line_start..line_start + BYTE_PER_LINE {
            if byte >= end {
                break;
            }
            let byte = unsafe { *(byte as *const u8) };
            let byte_msn = (byte >> 4) & 0x0F;
            let byte_lsn = byte & 0x0F;
            line[head] = hex_nibble(byte_msn);
            line[head + 1] = hex_nibble(byte_lsn);
            line[head + 2] = b' ';
            head += 3;
        }
        debug!(
            "{:016x}: {}",
            line_start,
            str::from_utf8(&line[..head]).unwrap()
        );
    }
}

fn hex_nibble(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        10..=15 => b'A' + nibble - 10,
        _ => unimplemented!(),
    }
}
