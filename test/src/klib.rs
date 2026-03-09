use {
    core::ptr,
    klib::mem_test::phys::midmem::test_exports::{BLOCK_SIZE, block::Block, size::MidFrameSize},
    std::boxed::Box,
    x64::mem::{
        addr::Address,
        frame::{FrameRange, size::Frame4KiB},
    },
};

static INIT_BLOCK: Block = Block::all_free();

fn make_block() -> Box<Block> {
    let mut uninit = Box::new_uninit();
    unsafe {
        ptr::copy(&INIT_BLOCK as *const Block, uninit.as_mut_ptr(), 1);
        uninit.assume_init()
    }
}

fn bounds(fr: FrameRange<Frame4KiB>) -> (usize, usize) {
    let start = fr.start().boundary().as_usize();
    (start, start + *fr.size())
}

fn drain(size: MidFrameSize) -> Box<Block> {
    let count = BLOCK_SIZE / size.size();
    let mut block = make_block();

    for i in 0..count {
        let frame = block
            .alloc(size)
            .unwrap_or_else(|| panic!("alloc #{i} of {count} failed for {size:?}"));
        assert_eq!(*frame.size(), size.size(), "wrong size on alloc #{i}");
    }

    block
}

#[test]
fn alloc_m8_sequential() {
    let mut block = make_block();

    for i in 0..4 {
        let frame = block
            .alloc(MidFrameSize::M8)
            .unwrap_or_else(|| panic!("M8 alloc #{i} failed"));
        assert_eq!(*frame.size(), MidFrameSize::M8.size());
        assert_eq!(
            *frame.start().boundary(),
            i * MidFrameSize::M8.size(),
            "M8 frame #{i} started at wrong offset"
        );
    }
}

#[test]
fn alloc_mixed_sizes_correct_offsets() {
    let mut block = make_block();

    let _m8_0 = block.alloc(MidFrameSize::M8).unwrap();
    let _m8_1 = block.alloc(MidFrameSize::M8).unwrap();
    let base = 2 * MidFrameSize::M8.size();

    let k4 = block.alloc(MidFrameSize::K4).unwrap();
    assert_eq!(*k4.size(), MidFrameSize::K4.size());
    assert_eq!(*k4.start().boundary(), base);

    let k4b = block.alloc(MidFrameSize::K4).unwrap();
    assert_eq!(*k4b.start().boundary(), base + MidFrameSize::K4.size());

    let k64 = block.alloc(MidFrameSize::K64).unwrap();
    assert_eq!(*k64.size(), MidFrameSize::K64.size());
    assert_eq!(*k64.start().boundary(), base + MidFrameSize::K64.size());

    let m2 = block.alloc(MidFrameSize::M2).unwrap();
    assert_eq!(*m2.size(), MidFrameSize::M2.size());
    assert_eq!(*m2.start().boundary(), base + MidFrameSize::M2.size());
}

#[test]
fn alloc_k4_no_overlap() {
    const COUNT: usize = MidFrameSize::M8.size() / MidFrameSize::K4.size() + 1;

    let mut block = make_block();
    let mut seen: Vec<(usize, usize)> = Vec::with_capacity(COUNT);

    for _ in 0..COUNT {
        let Some(fr) = block.alloc(MidFrameSize::K4) else {
            break;
        };
        let (s, e) = bounds(fr);
        for &(os, oe) in &seen {
            assert!(
                e <= os || s >= oe,
                "overlap: new [{s:#x},{e:#x}) vs existing [{os:#x},{oe:#x})"
            );
        }
        seen.push((s, e));
    }

    assert!(!seen.is_empty(), "no K4 allocations succeeded");
}

#[test]
fn split_frames_dont_overlap_prior_alloc() {
    let mut block = make_block();

    let m8 = block.alloc(MidFrameSize::M8).unwrap();
    let (m8s, m8e) = bounds(m8);

    for i in 0..512 {
        let Some(k4) = block.alloc(MidFrameSize::K4) else {
            break;
        };
        let (s, e) = bounds(k4);
        assert!(
            e <= m8s || s >= m8e,
            "K4 alloc #{i} [{s:#x},{e:#x}) overlapped M8 [{m8s:#x},{m8e:#x})"
        );
    }
}

#[test]
fn exhausted_block_returns_none() {
    let mut block = drain(MidFrameSize::M8);

    assert!(block.alloc(MidFrameSize::K4).is_none());
    assert!(block.alloc(MidFrameSize::K64).is_none());
    assert!(block.alloc(MidFrameSize::K128).is_none());
    assert!(block.alloc(MidFrameSize::M2).is_none());
    assert!(block.alloc(MidFrameSize::M8).is_none());
}

#[test]
fn overalloc_m2() {
    drain(MidFrameSize::M2);
}

#[test]
fn overalloc_k128() {
    drain(MidFrameSize::K128);
}

#[test]
fn overalloc_k64() {
    drain(MidFrameSize::K64);
}

#[test]
fn overalloc_k4() {
    drain(MidFrameSize::K4);
}

#[test]
fn coalesce_k4_to_m8() {
    let mut block = make_block();

    const K4_PER_M8: usize = MidFrameSize::M8.size() / MidFrameSize::K4.size();

    let frames: Vec<FrameRange<Frame4KiB>> = (0..K4_PER_M8)
        .map(|i| {
            block
                .alloc(MidFrameSize::K4)
                .unwrap_or_else(|| panic!("K4 alloc #{i} failed"))
        })
        .collect();

    assert_eq!(frames[0].start().boundary().as_usize(), 0);

    let total_m8 = BLOCK_SIZE / MidFrameSize::M8.size();
    assert!(
        block.free_count(MidFrameSize::M8) < total_m8,
        "M8 bitmap should show at least one consumed region after K4 drain"
    );

    for fr in frames {
        block.dealloc(fr);
    }

    assert_eq!(
        block.free_count(MidFrameSize::K4),
        0,
        "stale K4 bits after full coalesce"
    );
    assert_eq!(
        block.free_count(MidFrameSize::K64),
        0,
        "stale K64 bits after full coalesce"
    );
    assert_eq!(
        block.free_count(MidFrameSize::K128),
        0,
        "stale K128 bits after full coalesce"
    );
    assert_eq!(
        block.free_count(MidFrameSize::M2),
        0,
        "stale M2 bits after full coalesce"
    );

    assert_eq!(
        block.free_count(MidFrameSize::M8),
        total_m8,
        "M8 count should be restored to full after coalesce"
    );
}

#[test]
fn coalesce_only_when_buddy_group_complete() {
    let mut block = make_block();

    const K4_PER_K64: usize = MidFrameSize::K64.size() / MidFrameSize::K4.size();

    let mut frames: Vec<FrameRange<Frame4KiB>> = (0..K4_PER_K64)
        .map(|i| {
            block
                .alloc(MidFrameSize::K4)
                .unwrap_or_else(|| panic!("alloc #{i} failed"))
        })
        .collect();

    for fr in frames.drain(..K4_PER_K64 - 1) {
        block.dealloc(fr);
    }

    assert!(
        block.free_count(MidFrameSize::K4) > 0,
        "K4 bits should remain while buddy group is incomplete"
    );

    block.dealloc(frames.remove(0));

    assert_eq!(
        block.free_count(MidFrameSize::K4),
        0,
        "K4 bits should be gone after the last buddy is freed"
    );
}

#[test]
fn realloc_after_coalesce() {
    let mut block = make_block();

    const K4_PER_K64: usize = MidFrameSize::K64.size() / MidFrameSize::K4.size();

    let frames: Vec<_> = (0..K4_PER_K64)
        .map(|i| {
            block
                .alloc(MidFrameSize::K4)
                .unwrap_or_else(|| panic!("alloc #{i}"))
        })
        .collect();

    for fr in frames {
        block.dealloc(fr);
    }

    let frame = block.alloc(MidFrameSize::K4).unwrap();
    assert_eq!(frame.start().boundary().as_usize(), 0);
}

#[test]
fn fresh_block_free_counts() {
    let block = make_block();
    let total_m8 = BLOCK_SIZE / MidFrameSize::M8.size();

    assert_eq!(block.free_count(MidFrameSize::M8), total_m8);
    assert_eq!(block.free_count(MidFrameSize::M2), 0);
    assert_eq!(block.free_count(MidFrameSize::K128), 0);
    assert_eq!(block.free_count(MidFrameSize::K64), 0);
    assert_eq!(block.free_count(MidFrameSize::K4), 0);
}

#[test]
fn alloc_m8_decrements_free_count() {
    let mut block = make_block();
    let total_m8 = BLOCK_SIZE / MidFrameSize::M8.size();

    block.alloc(MidFrameSize::M8).unwrap();
    assert_eq!(block.free_count(MidFrameSize::M8), total_m8 - 1);
}

#[test]
fn alloc_k4_split_accounting() {
    let mut block = make_block();

    block.alloc(MidFrameSize::K4).unwrap();

    let total_m8 = BLOCK_SIZE / MidFrameSize::M8.size();
    assert_eq!(
        block.free_count(MidFrameSize::M8),
        total_m8 - 1,
        "split M8 should be removed from M8 free count"
    );

    let sub_m8_free = block.free_count(MidFrameSize::M2)
        + block.free_count(MidFrameSize::K128)
        + block.free_count(MidFrameSize::K64)
        + block.free_count(MidFrameSize::K4);
    assert!(
        sub_m8_free > 0,
        "split should leave free frames in sub-M8 bitmaps"
    );

    let total_block_k4 = BLOCK_SIZE / MidFrameSize::K4.size();
    let free_k4_equiv = block.free_count(MidFrameSize::M8)
        * (MidFrameSize::M8.size() / MidFrameSize::K4.size())
        + block.free_count(MidFrameSize::M2) * (MidFrameSize::M2.size() / MidFrameSize::K4.size())
        + block.free_count(MidFrameSize::K128)
            * (MidFrameSize::K128.size() / MidFrameSize::K4.size())
        + block.free_count(MidFrameSize::K64)
            * (MidFrameSize::K64.size() / MidFrameSize::K4.size())
        + block.free_count(MidFrameSize::K4);
    assert_eq!(
        free_k4_equiv,
        total_block_k4 - 1,
        "total free K4-equivalent frames should be (block_size / 4K) - 1"
    );
}
