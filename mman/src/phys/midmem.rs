mod freelist;
mod size;

use config::pmem::MIDMEM;

const MIDMEM_SIZE: usize = MIDMEM.size().as_usize();
