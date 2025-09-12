#![cfg(test)]

use {
    crate::phys::midmem::{BLOCK_SIZE, block::Block, freelist::Freelist, size::MidFrameSize},
    alloc::vec::Vec,
    core::ptr,
    spinlocks::mutex::{Mutex, MutexGuard},
    std::boxed::Box,
    x64::mem::{
        addr::{Address, PhysAddr},
        frame::{FrameRange, size::Frame4KiB},
    },
};

static INIT_BLOCK: Block = Block::new();

fn make_block() -> Box<Block> {
    let mut uninit_block = Box::new_uninit();

    unsafe {
        ptr::copy(&INIT_BLOCK as *const Block, uninit_block.as_mut_ptr(), 1);
    }

    unsafe { uninit_block.assume_init() }
}

fn frame_range_bounds(fr: &FrameRange<Frame4KiB>) -> (usize, usize) {
    let start = fr.start().boundary().as_usize();
    let end = start + *fr.size();
    (start, end)
}

#[test]
fn basic_block() {
    let mut block = make_block();

    let frame = block.alloc(MidFrameSize::M8).unwrap();
    assert_eq!(*frame.size(), 8 * 1024 * 1024);
    assert_eq!(*frame.start().boundary(), 0);

    let frame = block.alloc(MidFrameSize::M8).unwrap();
    assert_eq!(*frame.size(), MidFrameSize::M8.size());
    assert_eq!(*frame.start().boundary(), MidFrameSize::M8.size());

    let frame = block.alloc(MidFrameSize::K4).unwrap();
    assert_eq!(*frame.size(), MidFrameSize::K4.size());
    assert_eq!(*frame.start().boundary(), 2 * MidFrameSize::M8.size());

    let frame = block.alloc(MidFrameSize::K4).unwrap();
    assert_eq!(*frame.size(), MidFrameSize::K4.size());
    assert_eq!(
        *frame.start().boundary(),
        2 * MidFrameSize::M8.size() + MidFrameSize::K4.size()
    );

    let frame = block.alloc(MidFrameSize::K64).unwrap();
    assert_eq!(*frame.size(), MidFrameSize::K64.size());
    assert_eq!(
        *frame.start().boundary(),
        2 * MidFrameSize::M8.size() + MidFrameSize::K64.size()
    );

    let frame = block.alloc(MidFrameSize::M2).unwrap();
    assert_eq!(*frame.size(), MidFrameSize::M2.size());
    assert_eq!(
        *frame.start().boundary(),
        2 * MidFrameSize::M8.size() + MidFrameSize::M2.size()
    );
}

#[test]
fn alloc_many_k4_no_overlap() {
    const ALLOC_COUNT: usize = 4096 + 1; // Consumes 2 8M frames, plus partially a third one

    let mut block = make_block();
    let mut seen: Vec<(usize, usize)> = Vec::new();

    for _ in 0..ALLOC_COUNT {
        match block.alloc(MidFrameSize::K4) {
            Some(fr) => {
                let (s, e) = frame_range_bounds(&fr);
                for &(os, oe) in &seen {
                    assert!(
                        e <= os || s >= oe,
                        "overlap detected: new [{s:#x},{e:#x}) vs old [{os:#x},{oe:#x})"
                    );
                }
                seen.push((s, e));
            }
            None => break,
        }
    }

    assert!(!seen.is_empty(), "expected at least one K4 allocation");
}

#[test]
fn alloc_split_and_children_no_overlap() {
    let mut b = make_block();

    if let Some(big) = b.alloc(MidFrameSize::M8) {
        let (bs, be) = frame_range_bounds(&big);
        for _ in 0..512 {
            if let Some(k4) = b.alloc(MidFrameSize::K4) {
                let (s, e) = frame_range_bounds(&k4);
                assert!(
                    e <= bs || s >= be,
                    "K4 allocation overlapped pre-existing M8 allocation"
                );
            } else {
                break;
            }
        }
    } else {
        panic!(
            "unable to obtain an M8 allocation in fresh block (check initial bootmap/reserved ranges)"
        );
    }
}

#[test]
fn exhaustion_until_none() {
    let mut block = overalloc(MidFrameSize::M8);

    assert!(block.alloc(MidFrameSize::K4).is_none());
    assert!(block.alloc(MidFrameSize::K64).is_none());
    assert!(block.alloc(MidFrameSize::K128).is_none());
    assert!(block.alloc(MidFrameSize::M2).is_none());
    assert!(block.alloc(MidFrameSize::M8).is_none());
}

fn overalloc(size: MidFrameSize) -> Box<Block> {
    let alloc_count = BLOCK_SIZE / size.size();

    let mut block = make_block();

    for _ in 0..alloc_count {
        let frame = block.alloc(size).unwrap();
        assert_eq!(*frame.size(), size.size());
    }

    assert!(block.alloc(MidFrameSize::K4).is_none());

    block
}

#[test]
fn stress_word_boundaries() {
    let mut b = make_block();
    let mut allocations = Vec::new();

    for i in 0..50_000 {
        if let Some(fr) = b.alloc(MidFrameSize::K4) {
            allocations.push(frame_range_bounds(&fr));
        } else {
            break;
        }
    }

    allocations.sort_unstable();
    for pair in allocations.windows(2) {
        let (s0, e0) = pair[0];
        let (s1, e1) = pair[1];
        assert!(
            e0 <= s1,
            "adjacent allocations overlapped across words: [{:#x},{:#x}) vs [{:#x},{:#x})",
            s0,
            e0,
            s1,
            e1
        );
    }

    assert!(
        !allocations.is_empty(),
        "no allocations succeeded in stress test"
    );
}

#[test]
fn overalloc_m8() {
    overalloc(MidFrameSize::M8);
}

#[test]
fn overalloc_m2() {
    overalloc(MidFrameSize::M2);
}

#[test]
fn overalloc_k128() {
    overalloc(MidFrameSize::K128);
}

#[test]
fn overalloc_k64() {
    overalloc(MidFrameSize::K64);
}

#[test]
fn overalloc_k4() {
    overalloc(MidFrameSize::K4);
}
