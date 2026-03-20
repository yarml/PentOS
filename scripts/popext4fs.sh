#!/bin/sh

set -e
set -x

src_dir="$1"
img="$2"

while read -r path; do
    rel="${path#$src_dir}"
    [ -z "$rel" ] && continue

    if [ -d "$path" ]; then
        echo mkdir $rel
    else
        echo write $path $rel
    fi
done < <(find "$src_dir") | debugfs -w "$img"

