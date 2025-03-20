use crate::bootstage;
use common::mem::addr::PhysAddr;
use core::mem;
use core::mem::MaybeUninit;
use core::slice;
use uefi::boot;
use uefi::boot::AllocateType;
use uefi::boot::MemoryType;

pub struct PreBootAllocator;

// We alloc, we never dealloc, after all, everything is used freely by the kernel
impl PreBootAllocator {
    pub fn alloc_raw(&self, size: usize, align: usize, mtype: MemoryType) -> Option<PhysAddr> {
        if !bootstage::is_preboot() {
            return None;
        }
        if align > 4096 || !align.is_power_of_two() {
            return None;
        }
        let size = size.next_multiple_of(4096);
        let pg_count = size / 4096;
        Some(
            boot::allocate_pages(AllocateType::AnyPages, mtype, pg_count)
                .ok()?
                .as_ptr()
                .into(),
        )
    }
    pub fn alloc<T>(&self, init: T, mtype: MemoryType) -> Option<&'static mut T> {
        let size = core::mem::size_of::<T>();
        let align = core::mem::align_of::<T>();
        let start = self.alloc_raw(size, align, mtype)?;
        let ptr = start.as_mut_ptr::<T>();
        unsafe {
            // SAFETY: `start` is (over)aligned, and is not null
            ptr.write(init);
            Some(&mut *ptr)
        }
    }
    pub fn alloc_uninit<T>(&self, mtype: MemoryType) -> Option<&'static mut MaybeUninit<T>> {
        let size = core::mem::size_of::<MaybeUninit<T>>();
        let align = core::mem::align_of::<MaybeUninit<T>>();
        let start = self.alloc_raw(size, align, mtype)?;
        let ptr = start.as_mut_ptr::<MaybeUninit<T>>();
        Some(unsafe {
            // SAFETY: `start` is (over)aligned, and is not null
            &mut *ptr
        })
    }
    pub fn alloc_slice<T: Copy>(
        &self,
        len: usize,
        init: T,
        mtype: MemoryType,
    ) -> Option<&'static mut [T]> {
        let size = mem::size_of::<T>().checked_mul(len)?;
        let align = mem::align_of::<T>();
        let start = self.alloc_raw(size, align, mtype)?;
        let ptr = start.as_mut_ptr::<T>();
        unsafe {
            // SAFETY: `start` is (over)aligned, and is not null
            for i in 0..len {
                ptr.add(i).write(init);
            }
            Some(slice::from_raw_parts_mut(ptr, len))
        }
    }
}
