#!/bin/bash

DIR=$(mktemp -d)
QMS=$DIR/qms

QEMU_CMD=$(echo qemu-system-x86_64 \
    -debugcon stdio \
    -monitor unix:$QMS,server \
    -s -S \
    -smp 4 \
    -m 8G \
    -d unimp,guest_errors \
    -enable-kvm \
    -cpu qemu64-v1,pdpe1gb,pcid,invpcid \
    -drive if=pflash,format=raw,readonly=on,file=run/ovmf/code.fd \
    -drive if=pflash,format=raw,readonly=on,file=run/ovmf/vars.fd \
    -drive format=raw,file=fat:rw:run/esp \
)

GDB=gdb

tmux -f scripts/tmux.rc \
    new sh -c "sleep 1 && $GDB -x scripts/gdb.rc -tui" \; \
    splitp -h  "$QEMU_CMD | tee run/qemu.log 2>&1" \; \
    splitp -v sh -c "sleep 1 && socat -,echo=0,icanon=0 unix-connect:$QMS"

rm -rf $DIR
