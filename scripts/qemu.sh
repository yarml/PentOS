qemu_cmd() {
    echo qemu-system-x86_64 \
        -debugcon stdio \
        -smp 12 \
        -m 8G \
        -full-screen \
        -cpu qemu64-v1,pdpe1gb,pcid,invpcid,fsgsbase,x2apic,rdrand \
        -drive if=pflash,format=raw,readonly=on,file=run/ovmf/code.fd \
        -drive if=pflash,format=raw,readonly=on,file=run/ovmf/vars.fd \
        -drive format=raw,file=fat:rw:$1/esp \
        -device VGA,vgamem_mb=8,xres=1920,yres=1080
}
