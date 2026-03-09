qemu_cmd() {
    echo qemu-system-x86_64 \
        -debugcon stdio \
        -smp 2 \
        -m 8G \
        -cpu qemu64-v1,pdpe1gb,pcid,invpcid,fsgsbase \
        -drive if=pflash,format=raw,readonly=on,file=run/ovmf/code.fd \
        -drive if=pflash,format=raw,readonly=on,file=run/ovmf/vars.fd \
        -drive format=raw,file=fat:rw:$1/esp
}
