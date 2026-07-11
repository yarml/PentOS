use {
    crate::bootinfo::bootinfo,
    log::info,
    pci::adr::{BusAddress, DEV_PER_BUS},
    x64::mem::addr::Address,
};

pub(crate) fn init() {
    let bootinfo = bootinfo();
    let config_spaces = &**bootinfo.topology.pci_config_spaces;

    for cs in config_spaces {
        for bus in cs.bus_start..cs.bus_end {
            let bus_addr = BusAddress::new(cs.segment_group, bus);
            for dev in 0..DEV_PER_BUS {
                let dev_addr = bus_addr.dev(dev);
                let f0 = dev_addr.func0();
                let id_reg = f0.reg(0);
                let id_ptr = id_reg.start_virtaddr().as_ptr::<u32>();
                let id = unsafe { id_ptr.read_volatile() };
                let devid = id >> 16;
                let vendid = id & 0xFFFF;

                if vendid == 0xFFFF {
                    continue;
                }

                info!("VendorId: {vendid:04x}, DevId: {devid:04x}");
            }
        }
    }
}
