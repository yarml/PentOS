ENTRY(kernel_meta)

MEMORY {
    kernel (rwx) : ORIGIN = 0xfffffffc00000000, LENGTH = 16384M - 1 /* Kernel code/rodata/data has the last 16Gib of kernel space */
}

PHDRS {
    kernel_code PT_LOAD;
    kernel_rodata PT_LOAD;
    kernel_data PT_LOAD;
    kernel_bss PT_LOAD;
}

SECTIONS {
    .text : ALIGN(0x1000) { *(.text .text.*) } >kernel :kernel_code
    .rodata : ALIGN(0x1000) { *(.rodata .rodata.*) } >kernel :kernel_rodata    
    .data : ALIGN(0x1000) { *(.data .data.*) } >kernel :kernel_data
    .bss : ALIGN(0x1000) { *(.bss .bss.*) } >kernel :kernel_bss
}
