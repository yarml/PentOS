use {
    crate::{
        allocator::PostBootAllocator,
        segmentation::{self, GdtInfo},
        topology, virt_mmap,
    },
    config::{
        topology::hart::{DF_STACK_SIZE, KSTACK_SIZE, MAX_HART_COUNT, NMI_STACK_SIZE},
        vmem::{DF_STACK_REGION, KHART_INFO, KSTACK_REGION, KTLS_REGION, NMI_STACK_REGION},
    },
    core::{cmp::min, slice},
    elf::{Elf, SegmentType},
    log::debug,
    system::hart::HartInfo,
    utils::collections::smallvec::SmallVec,
    x64::{
        mem::{
            MemorySize,
            addr::{Address, PhysAddr, VirtAddr},
            frame::{
                Frame,
                size::{Frame4KiB, FrameSize},
            },
            page::{
                Page,
                size::{Page2MiB, Page4KiB, PageSize},
            },
            paging::PagingRootEntry,
        },
        msr::pat::MemoryType,
    },
};

pub struct KernelHartInfo {
    pub kernel_stacks: KernelStacks,
    pub tlss: SmallVec<VirtAddr, MAX_HART_COUNT>,
    pub hartinfos: SmallVec<VirtAddr, MAX_HART_COUNT>,
    pub gdt_info: GdtInfo,
}

pub struct KernelStacks {
    pub stacks: SmallVec<VirtAddr, MAX_HART_COUNT>,
    pub df_stacks: SmallVec<VirtAddr, MAX_HART_COUNT>,
    pub nmi_stacks: SmallVec<VirtAddr, MAX_HART_COUNT>,
}

pub struct KernelStackSet {
    pub stack: VirtAddr,
    pub df_stack: VirtAddr,
    pub nmi_stack: VirtAddr,
}

/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn alloc_and_map_hart_mem<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    kernel: &Elf<'static>,
) -> KernelHartInfo {
    let kernel_stacks = unsafe {
        // SAFETY: Guarenteed by caller
        alloc_and_map_stacks(map_root, allocator)
    };
    let tlss = unsafe {
        // SAFETY: Guarenteed by caller
        alloc_and_map_tls(map_root, allocator, kernel)
    };
    let hartinfos = unsafe {
        // SAFETY: Guanrenteed by caller
        alloc_and_map_hartinfo(map_root, allocator)
    };

    let gdt_info = segmentation::setup_gdt(allocator);

    KernelHartInfo {
        kernel_stacks,
        tlss,
        hartinfos,
        gdt_info,
    }
}

/// # Safety
/// Should be called only once, and in the BSP
unsafe fn alloc_and_map_stacks<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> KernelStacks {
    // We will allocate as many stacks as harts on the system
    // And leave gaps between them to cause a page fault if a stack ever runs out
    // The gaps will be as large as a single stack
    // We allocate stacks starting from the highest address, leaving a gap at the
    // end as well

    // Kernel stacks
    let mut stacks = SmallVec::new();
    let hart_count = topology::topology().harts.len();
    let mut current_stack = KSTACK_REGION.end();
    let pg_count = KSTACK_SIZE.as_usize() / Page2MiB::SIZE;

    // DF stacks
    let mut df_stacks = SmallVec::new();
    let mut current_df_stack = DF_STACK_REGION.end();
    let df_pg_count = DF_STACK_SIZE.as_usize() / Page4KiB::SIZE;

    // NMI stacks
    let mut nmi_stacks = SmallVec::new();
    let mut current_nmi_stack = NMI_STACK_REGION.end();
    let nmi_pg_count = NMI_STACK_SIZE.as_usize() / Page4KiB::SIZE;

    assert!(hart_count <= MAX_HART_COUNT);
    assert!(
        hart_count * 2 * KSTACK_SIZE.as_usize() < KSTACK_REGION.size().as_usize(),
        "Cannot fit kernel stacks for the number of harts"
    );

    for _ in 0..hart_count {
        let stack_ptr = alloc_and_map_kernel_stack::<Page2MiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            &mut current_stack,
            pg_count,
        );
        let df_stack_ptr = alloc_and_map_kernel_stack::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            &mut current_df_stack,
            df_pg_count,
        );
        let nmi_stack_ptr = alloc_and_map_kernel_stack::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            &mut current_nmi_stack,
            nmi_pg_count,
        );

        stacks
            .push(stack_ptr)
            .expect("Not enough kernel stack slots");
        df_stacks
            .push(df_stack_ptr)
            .expect("Not enoough DF stack slots");
        nmi_stacks
            .push(nmi_stack_ptr)
            .expect("Not enoough NMI stack slots");
    }

    KernelStacks {
        stacks,
        df_stacks,
        nmi_stacks,
    }
}

/// # Safety
/// Should be called once in the BSP
unsafe fn alloc_and_map_tls<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    kernel: &Elf<'static>,
) -> SmallVec<VirtAddr, MAX_HART_COUNT> {
    let hart_count = topology::topology().harts.len();
    assert!(hart_count <= MAX_HART_COUNT);

    let mut tls_iter = kernel
        .program_header
        .into_iter()
        .filter(|s| s.ty == SegmentType::ThreadLocalStorage);

    let Some(tls) = tls_iter.next() else {
        debug!("No Kernel TLS");
        return SmallVec::new();
    };

    if tls_iter.next().is_some() {
        panic!("Kernel has multiple TLS entries. Supports only a unique TLS within the kernel.");
    }

    if *tls.mem_size == 0 {
        debug!("No Kernel TLS");
        let mut storages = SmallVec::new();
        for _ in 0..hart_count {
            unsafe {
                // SAFETY: We checked that hart_count is less than the maximum
                storages.push(VirtAddr::null()).unwrap_unchecked();
            }
        }
        return storages;
    }

    let tls_size = tls.mem_size.next_multiple_of(Frame4KiB::SIZE);
    let total_size = tls_size * hart_count;

    debug!(
        "TLS {offset} {msize} * {hart_count} => {tsize}",
        offset = tls.offset,
        msize = tls.mem_size,
        tsize = MemorySize::new(total_size),
    );

    assert!(
        total_size <= *KTLS_REGION.size(),
        "Cannot fit kernel TLS for the number of harts"
    );

    let tls_init_image = unsafe {
        // SAFETY: trusting kernel binary for now
        // TODO: sanitize in file offsets
        slice::from_raw_parts(kernel.data.as_ptr().add(tls.offset as usize), tls.file_size)
    };

    let all_tls = unsafe {
        // SAFETY: All u8 are valid
        allocator
            .alloc_slice::<u8>(total_size)
            .expect("Couldn't allocate kernel TLS")
            .assume_init_mut()
    };

    let all_tls_virt = {
        let all_tls_phys: Frame = Frame::containing(PhysAddr::from(all_tls.as_ptr()));
        let all_tls_virt = Page::containing(KTLS_REGION.start());
        let pg_count = total_size / Frame4KiB::SIZE;
        virt_mmap::map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            all_tls_phys,
            all_tls_virt,
            pg_count,
            true,
            false,
            MemoryType::WriteBack,
        );
        all_tls_virt.boundary()
    };

    let mut storages = SmallVec::new();

    for i in 0..hart_count {
        let local_tls = &mut all_tls[i * tls_size..(i + 1) * tls_size];
        let copy_amount = min(tls_init_image.len(), *tls.mem_size);

        local_tls[..copy_amount].copy_from_slice(tls_init_image);
        local_tls[copy_amount..].fill(0);

        let local_tls_virt = all_tls_virt + i * tls_size;

        unsafe {
            // SAFETY: We checked that hart_count is less than the maximum
            storages.push(local_tls_virt).unwrap_unchecked();
        }
    }

    storages
}

/// # Safety
/// Should be called only once, and in the BSP
unsafe fn alloc_and_map_hartinfo<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> SmallVec<VirtAddr, MAX_HART_COUNT> {
    let hart_count = topology::topology().harts.len();
    assert!(hart_count <= MAX_HART_COUNT);

    let hartinfo_size = core::mem::size_of::<HartInfo>();
    assert!(
        hartinfo_size * hart_count <= *KHART_INFO.size(),
        "Could not fit hart info within its region"
    );

    let all_hartinfos = unsafe {
        allocator
            .alloc_slice::<u8>(hartinfo_size * hart_count)
            .expect("Could not allocate hart info memory")
            .assume_init_mut()
    };

    {
        let hartinfos_phys = Frame::containing(PhysAddr::from(all_hartinfos.as_ptr()));
        let hartinfos_virt = Page::containing(KHART_INFO.start());
        let pg_count = (hartinfo_size * hart_count).div_ceil(Frame4KiB::SIZE);
        virt_mmap::map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            hartinfos_phys,
            hartinfos_virt,
            pg_count,
            true,
            false,
            MemoryType::WriteBack,
        );
    }

    let mut hartinfos = SmallVec::new();

    for i in 0..hart_count {
        let hartinfo_virt = KHART_INFO.start() + i * hartinfo_size;

        unsafe {
            // SAFETY: Size checked before
            hartinfos.push(hartinfo_virt).unwrap_unchecked();
        }
    }

    hartinfos
}

fn alloc_and_map_kernel_stack<PS: PageSize, const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    current_stack: &mut VirtAddr,
    pg_count: usize,
) -> VirtAddr {
    let stack_ptr = *current_stack - PS::SIZE * pg_count;
    *current_stack -= 2 * PS::SIZE * pg_count;

    let phys_stack = unsafe {
        // SAFETY: All u8 are valid
        allocator
            .alloc_slice::<u8>(KSTACK_SIZE.as_usize())
            .expect("Couldn't allocate a kernel stack")
            .assume_init_mut()
    };
    phys_stack.fill(0); // Not really needed, but what we losing (time, time is money)

    let frame = Frame::containing(PhysAddr::new_panic(phys_stack.as_ptr() as usize));
    let page = Page::containing(stack_ptr - KSTACK_SIZE.as_usize());

    virt_mmap::map_many::<PS, ALLOCATOR_CAP>(
        map_root,
        allocator,
        frame,
        page,
        pg_count,
        true,  // WRITE
        false, // EXEC
        MemoryType::WriteBack,
    );
    stack_ptr
}

impl KernelStacks {
    pub fn pop_set(&mut self) -> KernelStackSet {
        let stack = self.stacks.pop().expect("Not enough kernel stacks");
        let df_stack = self.df_stacks.pop().expect("Not enough DF stacks");
        let nmi_stack = self.nmi_stacks.pop().expect("Not enough NMI stacks");
        KernelStackSet {
            stack,
            df_stack,
            nmi_stack,
        }
    }
}
