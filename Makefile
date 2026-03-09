ifeq ("$(wildcard target/debug/chef)","")
$(info Compiling chef: May take a while)
endif

.PHONY: nothing
nothing:

packages_names := $(shell cargo chef packages name)
packages_paths := $(shell cargo chef packages path)
packages := $(join $(packages_names),$(addprefix :, $(packages_paths)))

kernel_destination := $(shell cargo chef config install-kernel)
bootloader_destination := $(shell cargo chef config install-bootloader)

ovmf_target := run/ovmf/vars.fd run/ovmf/code.fd

BOOT_IMG_SIZE_MB := 64
BOOT_PART_END_MIB := 65
DISK_IMG_SIZE_MB := 128

.PHONY: check-all clippy
define package_build_recipe =
.PHONY: build-release-$(1) build-debug-$(1) check-$(1) clippy-$(1) doc-$(1)
build-debug-$(1):
	cd $(2) && cargo build -p $(1)
build-release-$(1):
	cd $(2) && cargo build -p $(1) --release
check-all: check-$(1)
check-$(1):
	@cd $(2) && cargo clippy --all-features --keep-going --quiet --message-format=json -p $(1)
clippy: clippy-$(1)
clippy-$(1):
	cd $(2) && cargo clippy --no-deps --all-features --keep-going -p $(1)
doc-$(1):
	cd $(2) && cargo doc --no-deps --all-features -p $(1)
endef

$(foreach package,$(packages), \
	$(eval $(call \
		package_build_recipe,$(word 1,$(subst :, ,$(package))),$(word 2,$(subst :, ,$(package))) \
	)) \
)

$(ovmf_target):
	cargo chef ovmf

.PHONY: image-debug image-release
image-debug: $(ovmf_target) build-debug-bootloader build-debug-kernel
	mkdir -p run/debug/esp/efi/boot
	cp target/uefi/debug/bootloader.efi run/debug/esp/efi/boot/bootx64.efi
	cp target/kernel/debug/kernel run/debug/esp/pentos.kernel

image-release: $(ovmf_target) build-release-bootloader build-release-kernel
	mkdir -p run/release/esp/efi/boot
	cp target/uefi/release/bootloader.efi run/release/esp/efi/boot/bootx64.efi
	cp target/kernel/release/kernel run/release/esp/pentos.kernel

.PHONY: run
run: image-release
	bash scripts/run.sh

.PHONY: debug
debug: image-debug
	bash scripts/debug.sh

.PHONY: install
install: build-release-bootloader build-release-kernel
	sudo cp target/uefi/release/bootloader.efi $(bootloader_destination)
	sudo cp target/kernel/release/kernel $(kernel_destination)

.PHONY: test
test:
	cargo test -p test

.PHONY: doc
doc:
	cargo doc --workspace --release --no-deps
	echo '<meta http-equiv="refresh" content="0; url=pentos/">' > target/doc/index.html

.PHONY: clean
clean:
	rm -rf run
	cargo clean

.PHONY: img-release img-debug
img-release: image-release
	mkdir -p run/release/partitions
	dd if=/dev/zero of=run/release/pentos.img bs=1M count=$(DISK_IMG_SIZE_MB)
	parted -s run/release/pentos.img mklabel gpt
	parted -s run/release/pentos.img mkpart BOOT fat32 1MiB $(BOOT_PART_END_MIB)MiB
	parted -s run/release/pentos.img set 1 esp on
	dd if=/dev/zero of=run/release/partitions/boot.img bs=1M count=$(BOOT_IMG_SIZE_MB)
	mkfs.fat -F 32 -n BOOT run/release/partitions/boot.img
	mtools -c mcopy -i run/release/partitions/boot.img -s run/release/esp/* ::
	dd if=run/release/partitions/boot.img of=run/release/pentos.img bs=1M seek=1 conv=notrunc

img-debug: image-debug
	mkdir -p run/debug/partitions
	dd if=/dev/zero of=run/debug/pentos.img bs=1M count=$(DISK_IMG_SIZE_MB)
	parted -s run/debug/pentos.img mklabel gpt
	parted -s run/debug/pentos.img mkpart BOOT fat32 1MiB $(BOOT_PART_END_MIB)MiB
	parted -s run/debug/pentos.img set 1 esp on
	dd if=/dev/zero of=run/debug/partitions/boot.img bs=1M count=$(BOOT_IMG_SIZE_MB)
	mkfs.fat -F 32 -n BOOT run/debug/partitions/boot.img
	mtools -c mcopy -i run/debug/partitions/boot.img -s run/debug/esp/* ::
	dd if=run/debug/partitions/boot.img of=run/debug/pentos.img bs=1M seek=1 conv=notrunc

.PHONY: vmdk-release vmdk-debug
vmdk-release: img-release
	rm -f run/release/pentos.vmdk
	VBoxManage convertfromraw run/release/pentos.img run/release/pentos.vmdk --format VMDK

vmdk-debug: img-debug
	rm -f run/debug/pentos.vmdk
	VBoxManage convertfromraw run/debug/pentos.img run/debug/pentos.vmdk --format VMDK

.PHONY: iso-release iso-debug
iso-release: img-release
	rm -f run/release/pentos.iso
	VBoxManage convertfromraw run/release/pentos.img run/release/pentos.iso --format RAW

iso-debug: img-debug
	rm -f run/debug/pentos.iso
	VBoxManage convertfromraw run/debug/pentos.img run/debug/pentos.iso --format RAW
