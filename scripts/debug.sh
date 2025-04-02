!#/bin/bash

DIR=$(mktemp -d)
QMS=$DIR/qms

QEMU_CMD=$(echo qemu-system-x86_64 \
    -debugcon stdio \
    -monitor unix:$QMS,server \
    -s -S \
    -smp 4 \
    -m 8G \
    -cpu qemu64,pdpe1gb=on \
    -drive if=pflash,format=raw,readonly=on,file=run/ovmf/code.fd \
    -drive if=pflash,format=raw,readonly=on,file=run/ovmf/vars.fd \
    -drive format=raw,file=fat:rw:run/esp \
)


tmux -f scripts/tmux.rc \
    new sh -c "sleep 1 && rust-gdb -x scripts/gdb.rc -tui" \; \
    splitp -h  $QEMU_CMD \; \
    splitp -v sh -c "sleep 1 && socat -,echo=0,icanon=0 unix-connect:$QMS"
