use {
    crate::ConfigSpace,
    core::{
        assert,
        fmt::{Debug, Display, Formatter, Result},
    },
    system::vmem,
    x64::mem::{
        addr::{Address, VirtAddr},
        page::size::{Page4KiB, PageSize},
    },
};

pub const FUNC_PER_DEV: usize = 8;
pub const DEV_PER_BUS: usize = 32;
pub const BUS_PER_SEGMENT: usize = 256;
pub const SEGMENT_COUNT: usize = 65536;

pub const FUNC_SIZE: usize = Page4KiB::SIZE;
pub const DEV_SIZE: usize = FUNC_PER_DEV * FUNC_SIZE;
pub const BUS_SIZE: usize = DEV_PER_BUS * DEV_SIZE;
pub const SEGMENT_SIZE: usize = BUS_PER_SEGMENT * BUS_SIZE;

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
        assert!(group < SEGMENT_COUNT);
        Self {
            group: group as u16,
        }
    }

    pub const fn bus(&self, bus: usize) -> BusAddress {
        assert!(bus < BUS_PER_SEGMENT);
        BusAddress {
            segment: *self,
            bus: bus as u8,
        }
    }

    pub const fn group(&self) -> usize {
        self.group as usize
    }

    pub const fn start_virtaddr(&self) -> VirtAddr {
        vmem::PCIE_REGION
            .start()
            .add_panic(self.group() * SEGMENT_SIZE)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(SEGMENT_SIZE)
    }
}

impl BusAddress {
    pub const fn new(group: usize, bus: usize) -> Self {
        assert!(bus < BUS_PER_SEGMENT);
        Self {
            segment: SegmentAddress::new(group),
            bus: bus as u8,
        }
    }

    pub const fn dev(&self, dev: usize) -> DeviceAddress {
        assert!(dev < DEV_PER_BUS);
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
        self.segment
            .start_virtaddr()
            .add_panic(self.bus() * BUS_SIZE)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(BUS_SIZE)
    }
}

impl DeviceAddress {
    pub const fn new(group: usize, bus: usize, device: usize) -> Self {
        assert!(device < DEV_PER_BUS);
        Self {
            bus: BusAddress::new(group, bus),
            device: device as u8,
        }
    }

    pub const fn func(&self, func: usize) -> FunctionAddress {
        assert!(func < FUNC_PER_DEV);
        FunctionAddress {
            device: *self,
            function: func as u8,
        }
    }
    pub const fn func0(&self) -> FunctionAddress {
        self.func(0)
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
        self.bus.start_virtaddr().add_panic(self.dev() * DEV_SIZE)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(DEV_SIZE)
    }
}

impl FunctionAddress {
    pub const fn new(group: usize, bus: usize, device: usize, func: usize) -> Self {
        assert!(func < FUNC_PER_DEV);
        Self {
            device: DeviceAddress::new(group, bus, device),
            function: func as u8,
        }
    }

    pub const fn reg(&self, offset: usize) -> RegisterAddress {
        assert!(offset < FUNC_SIZE);
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
            .add_panic(self.func() * FUNC_SIZE)
    }
    pub const fn end_virtaddr(&self) -> VirtAddr {
        self.start_virtaddr().add_panic(FUNC_SIZE)
    }

    pub const fn config_space(&self) -> ConfigSpace {
        unsafe { ConfigSpace::new(self.start_virtaddr()) }
    }
}

impl RegisterAddress {
    pub const fn new(group: usize, bus: usize, device: usize, func: usize, offset: usize) -> Self {
        assert!(offset < FUNC_SIZE);
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

impl Display for SegmentAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:02x}", self.group)
    }
}

impl Display for BusAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}#{:02x}", self.segment, self.bus)
    }
}

impl Display for DeviceAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}:{:02x}", self.bus, self.device)
    }
}

impl Display for FunctionAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}.{:02x}", self.device, self.function)
    }
}

impl Debug for SegmentAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self)
    }
}

impl Debug for BusAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self)
    }
}

impl Debug for DeviceAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self)
    }
}

impl Debug for FunctionAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self)
    }
}
