.PHONY: img-release img-debug
img-release: flat-release
	mkdir -p run/release/partitions
	dd if=/dev/zero of=run/release/pentos.img bs=1M count=$(DISK_IMG_SIZE_MB)
	parted -s run/release/pentos.img mklabel gpt
	parted -s run/release/pentos.img mkpart BOOT fat32 1MiB $(BOOT_PART_END_MIB)MiB
	parted -s run/release/pentos.img set 1 esp on
	dd if=/dev/zero of=run/release/partitions/boot.img bs=1M count=$(BOOT_IMG_SIZE_MB)
	mkfs.fat -F 32 -n BOOT run/release/partitions/boot.img
	mtools -c mcopy -i run/release/partitions/boot.img -s run/release/flat/boot/* ::
	dd if=run/release/partitions/boot.img of=run/release/pentos.img bs=1M seek=1 conv=notrunc

img-debug: flat-debug
	mkdir -p run/debug/partitions
	dd if=/dev/zero of=run/debug/pentos.img bs=1M count=$(DISK_IMG_SIZE_MB)
	parted -s run/debug/pentos.img mklabel gpt
	parted -s run/debug/pentos.img mkpart BOOT fat32 1MiB $(BOOT_PART_END_MIB)MiB
	parted -s run/debug/pentos.img set 1 esp on
	dd if=/dev/zero of=run/debug/partitions/boot.img bs=1M count=$(BOOT_IMG_SIZE_MB)
	mkfs.fat -F 32 -n BOOT run/debug/partitions/boot.img
	mtools -c mcopy -i run/debug/partitions/boot.img -s run/debug/flat/boot/* ::
	dd if=run/debug/partitions/boot.img of=run/debug/pentos.img bs=1M seek=1 conv=notrunc