mod test;

use {
    crate::phys::midmem::{BLOCK_SIZE, freelist::Freelist, size::MidFrameSize},
    debug::gdb_print,
    x64::mem::{
        addr::PhysAddr,
        frame::{
            Frame, FrameRange,
            size::{Frame4KiB, FrameSize},
        },
    },
};

macro_rules! bitmap_decl {
    ($size:ident) => {
        [u64; BLOCK_SIZE / MidFrameSize::$size.size() / core::mem::size_of::<u64>()]
    };
}

/// Manages a 64 * 8MiB (That's 512MiB) block of middle memory.
/// In middle memory there is a total of 8 blocks (since middle memory is 4GiB)
/// With the first block always having the lower 16MiB always marked as allocated
/// since those are managed by the lower memory allocator
/// Spans of memory that are not marked as usable memory by the bootloader are just marked as used
/// Bitmaps uses 0 for used region, and 1 for free region, this is so we can have the blocks in the BSS
/// region(not consuming ELF file space), and the system can then mark regions as free from UEFI map.
pub struct Block {
    k4: bitmap_decl!(K4),
    k64: bitmap_decl!(K64),
    k128: bitmap_decl!(K128),
    m2: bitmap_decl!(M2),
    m8: bitmap_decl!(M8),
    freelist: Freelist,
    base: PhysAddr,
}

impl Block {
    pub const fn new() -> Self {
        Self {
            k4: [u64::MAX; _],
            k64: [u64::MAX; _],
            k128: [u64::MAX; _],
            m2: [u64::MAX; _],
            m8: [u64::MAX; _],
            freelist: Freelist::new(),
            base: PhysAddr::MIN,
        }
    }
}

impl Block {
    pub fn alloc(&mut self, size: MidFrameSize) -> Option<FrameRange<Frame4KiB>> {
        gdb_print!("Bloc::alloc: begin");
        for current in size.into_iter().rev() {
            if let Some(mut frame) = self.freelist.pop(current) {
                let _freelist_frame_size = MidFrameSize::from_size(*frame.size());
                gdb_print!("Block::alloc: freelist {_freelist_frame_size:?} => {size:?}");
                for current in current.into_iter() {
                    // This is guarenteed to be true before the loop ends naturally
                    if current == size {
                        gdb_print!("Bloc::alloc => freelist {frame:?}");
                        return Some(frame);
                    }
                    if let Some(child_order) = current.child_order() {
                        let bitmap = self.getbitmap(child_order);
                        gdb_print!(
                            "Block::alloc: freelist splitting: current: {current:?} child_order: {child_order:?} child_order#:{}",
                            child_order.order()
                        );
                        let mut buddies = frame.split::<Frame4KiB>(child_order.order());
                        frame = buddies.next().unwrap();
                        let children_mask = current.children_mask().unwrap();
                        let (byte_location, primary_bitloc) = Self::bitlocation(frame);
                        // Mark all children as used, since they're now managed by the free list
                        // outside of the bitmap jurisdiction
                        // Remember 0 is used, 1 is free
                        bitmap[byte_location] &= !(children_mask << primary_bitloc);
                        for buddy in buddies {
                            self.freelist.push(buddy);
                        }
                    }
                }
            }
        }
        gdb_print!("Block::alloc: freelist empty");

        // Freelist was useless
        let mut range = 0..64;
        for current in MidFrameSize::M8 {
            let bitmap = self.getbitmap(current);

            // TODO: Check multiple bits at a time, that's why we're using a u64 not a u8
            // I'm just too tired now
            let (byteloc, bitloc, index) = range.find_map(|i| {
                let byteloc = i / 64;
                let bitloc = i % 64;
                (bitmap[byteloc] & (1 << bitloc) != 0).then_some((byteloc, bitloc, i))
            })?;

            // The frame we find is necessarily the primary in its buddy set
            // We will mark it alongside all its buddies as used, add its buddies to the freelist
            // And either return the primary, or split it further depeding on the request
            let (mask, buddy_count) = if let Some(parent) = current.parent_order() {
                (
                    parent.children_mask().unwrap(),
                    parent.children_count().unwrap(),
                )
            } else {
                (1, 1)
            };

            bitmap[byteloc] &= !(mask << bitloc);

            let frame = FrameRange::new(
                Frame::containing(self.base + current.size() * index),
                current.k4_count(),
            );

            for buddy in (1..buddy_count).rev() {
                let buddy_frame = frame + buddy;
                self.freelist.push(buddy_frame);
            }

            if current == size {
                gdb_print!("Block::aloc => bitmap {frame:?}");
                return Some(frame);
            }

            let children_count = current.children_count().unwrap();
            range = (index * children_count)..((index + 1) * children_count);
        }

        // Is this even reachable?
        None
    }

    fn bitlocation(frame: FrameRange<Frame4KiB>) -> (usize, usize) {
        let addr_mod_block_size = *frame.start().boundary() % BLOCK_SIZE;
        let order = MidFrameSize::from_size(*frame.size()).order();
        let frame_pos = addr_mod_block_size >> Frame4KiB::SHIFT >> order;
        (frame_pos / 64, frame_pos % 64)
    }
}

impl Block {
    fn getbitmap(&mut self, size: MidFrameSize) -> &mut [u64] {
        match size {
            MidFrameSize::K4 => &mut self.k4,
            MidFrameSize::K64 => &mut self.k64,
            MidFrameSize::K128 => &mut self.k128,
            MidFrameSize::M2 => &mut self.m2,
            MidFrameSize::M8 => &mut self.m8,
        }
    }
}
