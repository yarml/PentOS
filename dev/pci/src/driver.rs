use {
    crate::{
        CommonInfo,
        adr::{BusAddress, DEV_PER_BUS, FunctionAddress},
    },
    klib::bootinfo::bootinfo,
};

pub fn walk() -> impl Iterator<Item = (FunctionAddress, CommonInfo)> {
    let bootinfo = bootinfo();
    let config_spaces = &**bootinfo.topology.pci_config_spaces;

    gen move {
        for cs in config_spaces {
            for bus in cs.bus_start..=cs.bus_end {
                let bus_addr = BusAddress::new(cs.segment_group, bus);
                for dev in 0..DEV_PER_BUS {
                    let dev_addr = bus_addr.dev(dev);
                    for func in 0..8 {
                        let func_addr = dev_addr.func(func);
                        let config_space = func_addr.config_space();

                        let Some(info) = config_space.read_info() else {
                            continue;
                        };

                        yield (func_addr, info);
                    }
                }
            }
        }
    }
}
