use {
    alloc::{boxed::Box, sync::Arc, vec},
    block::BlockDevice,
    io::IoResult,
};

use crate::media::MediaType;

pub struct Fat {
    fat_type: FatType,
    fat: Box<[u32]>,

    fat_count: usize,
    fat_pg_first: usize,

    dirty_range: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

impl Fat {
    pub fn alloc(
        fat_type: FatType,
        page_size: usize,
        fat_pg_first: usize,
        fat_pg_count: usize,
        fat_count: usize,
    ) -> Self {
        let total_u32 = (fat_pg_count * page_size) / core::mem::size_of::<u32>();
        let fat = vec![0; total_u32].into_boxed_slice();

        Self {
            fat_type,
            fat_count,
            fat_pg_first,
            fat,
            dirty_range: None,
        }
    }
}

impl FatType {
    pub const fn fat_entry_bit_count(&self) -> usize {
        match self {
            FatType::Fat12 => 12,
            FatType::Fat16 => 16,
            FatType::Fat32 => 32,
        }
    }

    pub const fn eoc_mark_min(&self) -> usize {
        match self {
            FatType::Fat12 => 0x0FF8,
            FatType::Fat16 => 0xFFF8,
            FatType::Fat32 => 0x0FFFFFF8,
        }
    }
    pub const fn bad_mark(&self) -> usize {
        match self {
            FatType::Fat12 => 0x0FF7,
            FatType::Fat16 => 0xFFF7,
            FatType::Fat32 => 0x0FFFFFF7,
        }
    }
}

impl Fat {
    pub const fn set_media(&mut self, media: MediaType) {
        self.set_entry_raw(0, 0xFFFFFF00 | media.code() as u32);
    }
    pub const fn set_eoc(&mut self) {
        self.set_entry_raw(1, self.fat_type.eoc_mark_min() as u32);
    }

    pub const fn set_entry(&mut self, index: usize, value: u32) {
        let mask = self.mask();

        assert!(index >= 2);
        assert!(value & mask == value);

        self.set_entry_raw(index, value);
    }
    pub const fn get_entry(&self, index: usize) -> u32 {
        assert!(index >= 2);
        self.get_entry_raw(index)
    }

    pub const fn as_raw(&self) -> &[u8] {
        unsafe {
            &*core::ptr::from_raw_parts(
                self.fat.as_ptr(),
                self.fat.len() * core::mem::size_of::<u32>(),
            )
        }
    }
}

impl Fat {
    const fn mask(&self) -> u32 {
        1u32.wrapping_shl(self.fat_type.fat_entry_bit_count() as u32)
            .wrapping_sub(1)
    }

    const fn set_entry_raw(&mut self, index: usize, value: u32) {
        let bits = self.fat_type.fat_entry_bit_count();

        let mask = self.mask();
        let value = value & mask;

        let global_bit_index = index * bits;

        let u32_index = global_bit_index / u32::BITS as usize;
        let local_bit_index = global_bit_index % u32::BITS as usize;

        let carry_over = local_bit_index + bits > u32::BITS as usize;

        if !carry_over {
            self.fat[u32_index] =
                (self.fat[u32_index] & !(mask << local_bit_index)) | (value << local_bit_index);
            self.mark_dirty(
                core::mem::size_of::<u32>() * u32_index,
                core::mem::size_of::<u32>() * (u32_index + 1),
            );
        } else {
            let bits_lo = u32::BITS as usize - local_bit_index;
            let mask_lo = (1u32 << bits_lo) - 1;
            let bits_hi = bits - bits_lo;
            let mask_hi = (1u32 << bits_hi) - 1;
            self.fat[u32_index] = (self.fat[u32_index] & !(mask_lo << local_bit_index))
                | ((value & mask_lo) << local_bit_index);
            self.fat[u32_index + 1] =
                (self.fat[u32_index + 1] & !mask_hi) | ((value >> bits_lo) & mask_hi);
            self.mark_dirty(
                core::mem::size_of::<u32>() * u32_index,
                core::mem::size_of::<u32>() * (u32_index + 2),
            );
        }
    }

    const fn get_entry_raw(&self, index: usize) -> u32 {
        let bits = self.fat_type.fat_entry_bit_count();
        let mask = self.mask();
        let global_bit_index = index * bits;
        let u32_index = global_bit_index / u32::BITS as usize;
        let local_bit_index = global_bit_index % u32::BITS as usize;
        let carry_over = local_bit_index + bits > u32::BITS as usize;
        if !carry_over {
            (self.fat[u32_index] >> local_bit_index) & mask
        } else {
            let bits_lo = u32::BITS as usize - local_bit_index;
            let bits_hi = bits - bits_lo;
            let mask_hi = (1u32 << bits_hi) - 1;
            let value_lo = self.fat[u32_index] >> local_bit_index;
            let valu_hi = self.fat[u32_index + 1] & mask_hi;
            value_lo | (valu_hi << bits_lo)
        }
    }

    const fn mark_dirty(&mut self, start: usize, end: usize) {
        self.dirty_range = Some(match self.dirty_range {
            None => (start, end),
            Some((s, e)) => (s.min(start), e.max(end)),
        });
    }
}

impl Fat {
    /// writes to device the dirty range
    /// # Notes
    /// Does not flush the underlying device
    pub async fn flush(&mut self, device: Arc<dyn BlockDevice>) -> IoResult<()> {
        let Some((start, end)) = self.dirty_range else {
            return Ok(());
        };

        let page_size = device.dimensions().page_size;
        let first_pg = start / page_size;
        let last_pg = end.div_ceil(page_size);

        let fat_pg_count = (self.fat.len() * core::mem::size_of::<u32>() / page_size) as u64;

        let raw = self.as_raw();
        let buf = &raw[first_pg * page_size..last_pg * page_size];

        for i in 0..self.fat_count as u64 {
            let fat_pg_start = i * fat_pg_count;
            device
                .write_pages(
                    fat_pg_start + first_pg as u64 + self.fat_pg_first as u64,
                    buf,
                )
                .await?;
        }

        self.dirty_range = None;
        Ok(())
    }
}
