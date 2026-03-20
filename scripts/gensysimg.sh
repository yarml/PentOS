set -e
set -x

img_dir=$1
boot_img_size_mb=$2
main_img_size_mb=$3

EFI_SYSTEM_TYPE=C12A7328-F81F-11D2-BA4B-00A0C93EC93B
LINUX_FS_TYPE=0FC63DAF-8483-4772-8E79-3D69D8477DE4

sfdisk $img_dir/pentos.img <<EOF
label: gpt
name=BOOT, size=${boot_img_size_mb}MiB, type=$EFI_SYSTEM_TYPE
name=MAIN, size=${main_img_size_mb}MiB, type=$LINUX_FS_TYPE
EOF

sfdisk -J $img_dir/pentos.img | \
    python3 scripts/extract_offset.py | \
    while read node offset; do \
        case $node in \
            *1) dd if=$img_dir/boot.img of=$img_dir/pentos.img bs=1M seek=$offset conv=notrunc ;; \
            *2) dd if=$img_dir/main.img of=$img_dir/pentos.img bs=1M seek=$offset conv=notrunc ;; \
        esac \
    done

