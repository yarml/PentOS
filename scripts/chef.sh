#!/bin/sh

if [ ! -f target/debug/chef ]; then
    cargo build --quiet --bin chef
fi

target/debug/chef $@
