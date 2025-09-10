mod block;
mod freelist;
mod size;

use config::pmem::MIDMEM;

const BLOCK_SIZE: usize = 512 * 1024 * 1024;
