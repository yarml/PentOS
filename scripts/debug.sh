#!/bin/bash

DIR=$(mktemp -d)
QMS=$DIR/qms

source scripts/qemu.sh

QEMU_CMD=$(echo $(qemu_cmd run/debug) \
    -monitor unix:$QMS,server \
    -d unimp,guest_errors \
    -s -S \
)

tmux -f scripts/tmux.rc \
    new sh -c "sleep 1 && gdb -x scripts/gdb.rc -tui" \; \
    splitp -h  "$QEMU_CMD | tee run/qemu.log 2>&1" \; \
    splitp -v sh -c "sleep 1 && socat -,echo=0,icanon=0 unix-connect:$QMS"

rm -rf $DIR
