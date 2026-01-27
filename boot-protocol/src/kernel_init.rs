pub type KernelInitFn = extern "sysv64" fn(is_bsp: bool) -> !;
