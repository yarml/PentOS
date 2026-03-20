define make_disk_img
	mkdir -p $(1)/img
	dd if=/dev/zero of=$(1)/img/pentos.img bs=1M count=$(DISK_IMG_SIZE_MB)
	bash scripts/gensysimg.sh $(1)/img $(BOOT_IMG_SIZE_MB) $(MAIN_IMG_SIZE_MB)
endef

run/release/img/boot.img: $(FLAT_BOOT_RELEASE_FILES)
	mkdir -p $(dir $@)
	dd if=/dev/zero of=$@ bs=1M count=$(BOOT_IMG_SIZE_MB)
	mkfs.fat -F 32 -n BOOT $@
	mtools -c mcopy -i $@ -s run/release/flat/boot/* ::

run/debug/img/boot.img: $(FLAT_BOOT_DEBUG_FILES)
	mkdir -p $(dir $@)
	dd if=/dev/zero of=$@ bs=1M count=$(BOOT_IMG_SIZE_MB)
	mkfs.fat -F 32 -n BOOT $@
	mtools -c mcopy -i $@ -s run/debug/flat/boot/* ::

run/release/img/main.img: $(FLAT_MAIN_RELEASE_FILES)
	mkdir -p $(dir $@)
	dd if=/dev/zero of=$@ bs=1M count=$(MAIN_IMG_SIZE_MB)
	mkfs.ext4 $@
	bash scripts/popext4fs.sh run/release/flat/main $@

run/debug/img/main.img: $(FLAT_MAIN_DEBUG_FILES)
	mkdir -p $(dir $@)
	dd if=/dev/zero of=$@ bs=1M count=$(MAIN_IMG_SIZE_MB)
	mkfs.ext4 $@
	bash scripts/popext4fs.sh run/debug/flat/main $@

run/release/img/pentos.img: run/release/img/boot.img run/release/img/main.img
	$(call make_disk_img,run/release)

run/debug/img/pentos.img: run/debug/img/boot.img run/debug/img/main.img
	$(call make_disk_img,run/debug)

.PHONY: img-boot-release img-boot-debug img-main-release img-main-debug img-release img-debug

img-boot-release: run/release/img/boot.img
img-boot-debug: run/debug/img/boot.img
img-main-release: run/release/img/main.img
img-main-debug: run/debug/img/main.img
img-release: run/release/img/pentos.img
img-debug: run/debug/img/pentos.img
