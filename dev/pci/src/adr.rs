use {
    core::assert,
    system::vmem,
    x64::mem::{
        addr::{Address, VirtAddr},
        page::size::{Page4KiB, PageSize},
    },
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SegmentAddress {
    group: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BusAddress {
    segment: SegmentAddress,
    bus: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DeviceAddress {
    bus: BusAddress,
    device: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FunctionAddress {
    device: DeviceAddress,
    function: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RegisterAddress {
    function: FunctionAddress,
    offset: u16,
}

impl SegmentAddress {
    pub const fn new(group: usize) -> Self {
        assert!(group < 65536);
        Self {
            group: group as u16,
        }
    }

    pub const fn bus(&self, bus: usize) -> BusAddress {
        BusAddress {
            segment: *self,
            bus: bus as u8,
        }
    }

    pub const fn group(&self) -> usize {
        self.group as usize
    }

    pub const fn start_virtaddr(&self) -> VirtAddr {
        vmem::PCIE_REGION.start().add_panic(self.group() * 65536)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr()
            .add_panic(256 * 32 * 8 * Page4KiB::SIZE)
    }
}

impl BusAddress {
    pub const fn new(group: usize, bus: usize) -> Self {
        Self {
            segment: SegmentAddress::new(group),
            bus: bus as u8,
        }
    }

    pub const fn dev(&self, dev: usize) -> DeviceAddress {
        DeviceAddress {
            bus: *self,
            device: dev as u8,
        }
    }

    pub const fn group(&self) -> usize {
        self.segment.group()
    }
    pub const fn bus(&self) -> usize {
        self.bus as usize
    }

    pub const fn start_virtaddr(&self) -> VirtAddr {
        self.segment.start_virtaddr().add_panic(self.bus() * 256)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(32 * 8 * Page4KiB::SIZE)
    }
}

impl DeviceAddress {
    pub const fn new(group: usize, bus: usize, device: usize) -> Self {
        Self {
            bus: BusAddress::new(group, bus),
            device: device as u8,
        }
    }

    pub const fn func(&self, func: usize) -> FunctionAddress {
        FunctionAddress {
            device: *self,
            function: func as u8,
        }
    }

    pub const fn group(&self) -> usize {
        self.bus.segment.group()
    }
    pub const fn bus(&self) -> usize {
        self.bus.bus()
    }
    pub const fn dev(&self) -> usize {
        self.device as usize
    }

    pub const fn start_virtaddr(&self) -> VirtAddr {
        self.bus.start_virtaddr().add_panic(self.dev() * 8)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(8 * Page4KiB::SIZE)
    }
}

impl FunctionAddress {
    pub const fn new(group: usize, bus: usize, device: usize, func: usize) -> Self {
        Self {
            device: DeviceAddress::new(group, bus, device),
            function: func as u8,
        }
    }

    pub const fn reg(&self, offset: usize) -> RegisterAddress {
        RegisterAddress {
            function: *self,
            offset: offset as u16,
        }
    }

    pub const fn group(&self) -> usize {
        self.device.group()
    }
    pub const fn bus(&self) -> usize {
        self.device.bus()
    }
    pub const fn dev(&self) -> usize {
        self.device.dev()
    }
    pub const fn func(&self) -> usize {
        self.function as usize
    }

    pub const fn start_virtaddr(&self) -> VirtAddr {
        self.device
            .start_virtaddr()
            .add_panic(self.func() * Page4KiB::SIZE)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(Page4KiB::SIZE)
    }
}

impl RegisterAddress {
    pub const fn new(group: usize, bus: usize, device: usize, func: usize, offset: usize) -> Self {
        Self {
            function: FunctionAddress::new(group, bus, device, func),
            offset: offset as u16,
        }
    }

    pub const fn group(&self) -> usize {
        self.function.group()
    }
    pub const fn bus(&self) -> usize {
        self.function.bus()
    }
    pub const fn dev(&self) -> usize {
        self.function.dev()
    }
    pub const fn func(&self) -> usize {
        self.function.func()
    }
    pub const fn offset(&self) -> usize {
        self.offset as usize
    }

    pub const fn start_virtaddr(&self) -> VirtAddr {
        self.function
            .start_virtaddr()
            .add_panic(self.offset as usize)
    }
}
