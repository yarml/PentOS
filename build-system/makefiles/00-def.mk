packages_names := $(shell bash scripts/chef.sh packages name)
packages_paths := $(shell bash scripts/chef.sh packages path)
packages := $(join $(packages_names),$(addprefix :, $(packages_paths)))

packages_userbin := $(shell bash scripts/chef.sh packages userbin)

kernel_destination := $(shell bash scripts/chef.sh config install-kernel)
bootloader_destination := $(shell bash scripts/chef.sh config install-bootloader)

ovmf_target := run/ovmf/vars.fd run/ovmf/code.fd

font_target := run/font.psf

BOOTLOADER_RELEASE := target/uefi/release/bootloader.efi
KERNEL_RELEASE := target/kernel/release/kernel
BOOTLOADER_DEBUG := target/uefi/debug/bootloader.efi
KERNEL_DEBUG := target/kernel/debug/kernel

BOOT_IMG_SIZE_MB := 64
MAIN_IMG_SIZE_MB := 128
DISK_IMG_SIZE_MB := 256

USERBIN_RELEASE := $(foreach p,$(packages_userbin),target/user/release/$(notdir $(p)))
USERBIN_DEBUG := $(foreach p,$(packages_userbin),target/user/debug/$(notdir $(p)))

.PHONY: nothing
nothing:

.PHONY: clean
clean:
	rm -rf run
	cargo clean
