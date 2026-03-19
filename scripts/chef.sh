#!/bin/sh

if [ ! -f target/debug/chef ]; then
    echo Building 'chef': may take a while
    cargo build --quiet --bin chef
fi

target/debug/chef $@
