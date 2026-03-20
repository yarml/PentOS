#!/bin/sh

if [ ! -f target/debug/chef ]; then
    echo Building 'chef': may take a while >&2
    cargo build --quiet --bin chef
fi

target/debug/chef $@
