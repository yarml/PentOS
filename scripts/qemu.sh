qemu_cmd() {
    echo qemu-system-x86_64 \
        -debugcon stdio \
        -smp $(bash scripts/chef.sh config qemu-numcores) \
        -m $(bash scripts/chef.sh config qemu-mem) \
        -full-screen \
        -cpu qemu64-v1,pdpe1gb,pcid,invpcid,fsgsbase,x2apic,rdrand \
        -drive if=pflash,format=raw,readonly=on,file=run/ovmf/code.fd \
        -drive if=pflash,format=raw,file=run/ovmf/vars.fd \
        -drive format=raw,file=$1/img/pentos.img \
        -device VGA,vgamem_mb=$(bash scripts/chef.sh config qemu-vgamem_mb),xres=$(bash scripts/chef.sh config qemu-xres),yres=$(bash scripts/chef.sh config qemu-yres)
}
