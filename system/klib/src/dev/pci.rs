use {
    crate::bootinfo::bootinfo,
    log::info,
    pci::adr::{BusAddress, DEV_PER_BUS},
};

pub(crate) fn init() {
    let bootinfo = bootinfo();
    let config_spaces = &**bootinfo.topology.pci_config_spaces;

    for cs in config_spaces {
        for bus in cs.bus_start..=cs.bus_end {
            let bus_addr = BusAddress::new(cs.segment_group, bus);
            for dev in 0..DEV_PER_BUS {
                let dev_addr = bus_addr.dev(dev);
                for func in 0..8 {
                    let func_addr = dev_addr.func(func);
                    let config_space = func_addr.config_space();

                    let Some(info) = config_space.read_info() else {
                        if func == 0 {
                            break;
                        } else {
                            continue;
                        }
                    };

                    info!("{func_addr}: {info}");

                    if !info.multifunction {
                        break;
                    }
                }
            }
        }
    }
}
