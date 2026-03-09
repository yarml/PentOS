//! Buddy allocator block for middle memory.
//!
//! # Design
//!
//! Each [`Block`] manages exactly 512 MiB of physical memory.
//! The block is subdivided into frames at five levels:
//!
//! - M8:   64 entry / block
//! - M2:   256 entry / block
//! - K128: 4 096 entry / block
//! - K64:  8 192 entry / block
//! - K4:   131 072 entry / block
//!
//! ## Invariant:
//!
//! A `1` bit means the frame at that position is **free**; `0` means
//! **allocated** (or unusable RAM).  Bitmaps start all-zero and
//! frames are marked free by the physical memory initialiser via
//! [`Block::dealloc`].
//!
//! ## Allocation
//!
//! 1. Search the bitmap for the requested size.  Use `u64::trailing_zeros` on
//!    each word to find the first free bit in O(words).
//! 2. If no frame of the right size is free, search the next larger size,
//!    allocate that, split it giving the first part to the caller and mark the
//!    other parts free in the smaller bitmap.  Repeat until the requested size
//!    is reached.
//! 3. If no frame of any size is free, return `None`.
//!
//! ## Deallocation & coalescing
//!
//! 1. Mark the frame free in its bitmap.
//! 2. Identify the sibling group: all frames of the same size that share the
//!    same parent. The group size varies by level (e.g. 4 M2s per M8, 16 K4s
//!    per K64).
//! 3. If every frame in the sibling group is now free, clear them all from the
//!    current bitmap and recurse upward, marking their parent free at the next
//!    level. This gives O(log n) coalescing per deallocation with no deferred
//!    work.

use {
    crate::mem::phys::midmem::{BLOCK_SIZE, size::MidFrameSize},
    x64::mem::{
        addr::{Address, PhysAddr},
        frame::{Frame, FrameRange, size::Frame4KiB},
    },
};

macro_rules! bitmap_len {
    ($size:ident) => {
        BLOCK_SIZE / MidFrameSize::$size.size() / u64::BITS as usize
    };
}

/// Manages one 512 MiB aligned region of physical memory.
///
/// Bitmaps sit in the BSS section (all-zero at boot).  Frames are exposed to
/// the allocator one-by-one through [`Block::dealloc`] during physical memory
/// initialisation.
pub struct Block {
    k4: [u64; bitmap_len!(K4)],
    k64: [u64; bitmap_len!(K64)],
    k128: [u64; bitmap_len!(K128)],
    m2: [u64; bitmap_len!(M2)],
    m8: [u64; bitmap_len!(M8)],

    base: PhysAddr,
}

impl Block {
    /// All-zero (everything allocated / unavailable). Suitable for BSS.
    pub const fn zero() -> Self {
        Self {
            k4: [0; bitmap_len!(K4)],
            k64: [0; bitmap_len!(K64)],
            k128: [0; bitmap_len!(K128)],
            m2: [0; bitmap_len!(M2)],
            m8: [0; bitmap_len!(M8)],
            base: PhysAddr::null(),
        }
    }

    /// All-free. Only used in unit tests.
    #[cfg(feature = "test")]
    pub const fn all_free() -> Self {
        Self {
            k4: [0; bitmap_len!(K4)],
            k64: [0; bitmap_len!(K64)],
            k128: [0; bitmap_len!(K128)],
            m2: [0; bitmap_len!(M2)],
            m8: [u64::MAX; bitmap_len!(M8)],
            base: PhysAddr::null(),
        }
    }
}

impl Block {
    /// Set the physical base address for this block.
    pub fn init(&mut self, base: PhysAddr) {
        self.base = base;
    }
}

impl Block {
    /// Allocate one frame of the requested size, or `None` if the block is
    /// exhausted.
    pub fn alloc(&mut self, size: MidFrameSize) -> Option<FrameRange<Frame4KiB>> {
        let found_size = {
            let mut found = None;
            for candidate in size.into_iter().rev() {
                if self.bitmap_find_free(candidate).is_some() {
                    found = Some(candidate);
                    break;
                }
            }
            found?
        };

        // Allocate at `found_size`, then split down to `size`.
        let index = self.bitmap_find_free(found_size).unwrap();
        self.bitmap_clear(found_size, index);

        self.split_to(found_size, index, size)
    }

    /// Return a frame to the block. Coalesces with the sibling group if
    /// possible, propagating upward.
    pub fn dealloc(&mut self, frame: FrameRange<Frame4KiB>) {
        let size = MidFrameSize::from_size(*frame.size());
        let index = self.frame_index(size, frame);
        self.free_and_coalesce(size, index);
    }
}

impl Block {
    #[inline]
    fn frame_index(&self, size: MidFrameSize, frame: FrameRange<Frame4KiB>) -> usize {
        let offset = *frame.start().boundary() % BLOCK_SIZE;
        offset / size.size()
    }

    #[inline]
    fn bitmap_find_free(&self, size: MidFrameSize) -> Option<usize> {
        let bitmap = self.bitmap_ref(size);
        for (word_idx, &word) in bitmap.iter().enumerate() {
            if word != 0 {
                let bit = word.trailing_zeros() as usize;
                return Some(word_idx * u64::BITS as usize + bit);
            }
        }
        None
    }

    #[inline]
    fn bitmap_clear(&mut self, size: MidFrameSize, index: usize) {
        let bitmap = self.bitmap_mut(size);
        bitmap[index / 64] &= !(1u64 << (index % 64));
    }

    #[inline]
    fn bitmap_set(&mut self, size: MidFrameSize, index: usize) {
        let bitmap = self.bitmap_mut(size);
        bitmap[index / 64] |= 1u64 << (index % 64);
    }

    #[inline]
    fn bitmap_is_free(&self, size: MidFrameSize, index: usize) -> bool {
        let bitmap = self.bitmap_ref(size);
        bitmap[index / 64] & (1u64 << (index % 64)) != 0
    }

    fn bitmap_ref(&self, size: MidFrameSize) -> &[u64] {
        match size {
            MidFrameSize::K4 => &self.k4,
            MidFrameSize::K64 => &self.k64,
            MidFrameSize::K128 => &self.k128,
            MidFrameSize::M2 => &self.m2,
            MidFrameSize::M8 => &self.m8,
        }
    }

    fn bitmap_mut(&mut self, size: MidFrameSize) -> &mut [u64] {
        match size {
            MidFrameSize::K4 => &mut self.k4,
            MidFrameSize::K64 => &mut self.k64,
            MidFrameSize::K128 => &mut self.k128,
            MidFrameSize::M2 => &mut self.m2,
            MidFrameSize::M8 => &mut self.m8,
        }
    }
}

impl Block {
    /// Given a frame at `found_size` with linear index `found_index`, split it
    /// down to `target_size`, marking the unwanted halves free at each level.
    /// Returns the frame range for the caller.
    fn split_to(
        &mut self,
        found_size: MidFrameSize,
        found_index: usize,
        target_size: MidFrameSize,
    ) -> Option<FrameRange<Frame4KiB>> {
        let mut current_size = found_size;
        let mut current_index = found_index;

        loop {
            if current_size == target_size {
                let offset = current_index * current_size.size();
                let frame = FrameRange::new(
                    Frame::containing(self.base + offset),
                    current_size.k4_count(),
                );
                return Some(frame);
            }

            let child_size = current_size.child_order()?;
            let children_per_parent = current_size.children_count()?;

            let first_child = current_index * children_per_parent;

            for sibling in 1..children_per_parent {
                self.bitmap_set(child_size, first_child + sibling);
            }

            current_size = child_size;
            current_index = first_child;
        }
    }

    /// Mark `index` free at `size`, then attempt to coalesce with its sibling group,
    /// propagating upward as far as possible.
    fn free_and_coalesce(&mut self, size: MidFrameSize, index: usize) {
        self.bitmap_set(size, index);

        // M8, no more coalescing to do
        let Some(parent_size) = size.parent_order() else {
            return;
        };

        let children_per_parent = parent_size.children_count().unwrap();
        let group_start = (index / children_per_parent) * children_per_parent;

        let all_free =
            (group_start..group_start + children_per_parent).all(|i| self.bitmap_is_free(size, i));

        if all_free {
            for i in group_start..group_start + children_per_parent {
                self.bitmap_clear(size, i);
            }
            let parent_index = group_start / children_per_parent;
            self.free_and_coalesce(parent_size, parent_index);
        }
    }
}

#[cfg(feature = "test")]
impl Block {
    pub fn free_count(&self, size: MidFrameSize) -> usize {
        self.bitmap_ref(size)
            .iter()
            .map(|w| w.count_ones() as usize)
            .sum()
    }
}
