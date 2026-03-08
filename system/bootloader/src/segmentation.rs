use {
    crate::{allocator::PostBootAllocator, topology},
    config::{topology::hart::MAX_HART_COUNT, vmem::PHYSICAL_MAPPING_REGION},
    core::mem::MaybeUninit,
    utils::collections::smallvec::SmallVec,
    x64::{
        mem::{
            addr::{Address, PhysAddr, VirtAddr},
            segmentation::{
                GlobalDescriptorTable, descriptor::SegmentDescriptor, selector::SegmentSelector,
                task_state::TaskStateSegment,
            },
        },
        prot::PrivilegeLevel,
    },
};

// Null + Kernel Code&Data + User Code&Data + TSS/hart
const TOTAL_GDT_LEN: usize = 5 + MAX_HART_COUNT;

pub type Gdt = GlobalDescriptorTable<TOTAL_GDT_LEN>;

pub struct GdtInfo {
    pub gdt: &'static Gdt,
    pub null: SegmentSelector,
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub user_data: SegmentSelector,
    pub tss_table: SmallVec<(SegmentSelector, VirtAddr), MAX_HART_COUNT>,
}

pub fn setup_gdt<const ALLOCATOR_CAP: usize>(
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> GdtInfo {
    let gdt = allocator
        .alloc(GlobalDescriptorTable::<TOTAL_GDT_LEN>::empty())
        .expect("Could not allocate system GDT");

    let hart_count = topology::topology().harts.len();

    let null = gdt.push(SegmentDescriptor::Null);
    let kernel_code = gdt.push(SegmentDescriptor::AccessSegment {
        exec: true,
        dpl: PrivilegeLevel::Kernel,
    });
    let kernel_data = gdt.push(SegmentDescriptor::AccessSegment {
        exec: false,
        dpl: PrivilegeLevel::Kernel,
    });

    let user_code = gdt.push(SegmentDescriptor::AccessSegment {
        exec: true,
        dpl: PrivilegeLevel::User,
    });
    let user_data = gdt.push(SegmentDescriptor::AccessSegment {
        exec: false,
        dpl: PrivilegeLevel::User,
    });

    let all_tss: &[TaskStateSegment] = unsafe {
        // SAFETY: TSS filled with 0s is valid as long as its not used
        // Which is the case here, no hart will use their own TSS before filling it
        let slice = allocator
            .alloc_slice(hart_count)
            .expect("Could not allocate TSS segments");
        slice.fill(MaybeUninit::zeroed());
        slice.assume_init_ref()
    };

    let mut tss_table = SmallVec::new();

    for tss_ref in all_tss {
        let phys_addr = PhysAddr::from(tss_ref as *const TaskStateSegment);
        let virt_addr = phys_addr.to_virt_with_offset(PHYSICAL_MAPPING_REGION.start().as_usize());
        let selector = gdt.push(SegmentDescriptor::TaskStateSegment { base: virt_addr });
        tss_table
            .push((selector, virt_addr))
            .expect("Not enough TSS selector spots");
    }

    GdtInfo {
        gdt,
        null,
        kernel_code,
        kernel_data,
        user_code,
        user_data,
        tss_table,
    }
}
