packages_names := $(shell bash scripts/chef.sh packages name)
packages_paths := $(shell bash scripts/chef.sh packages path)
packages := $(join $(packages_names),$(addprefix :, $(packages_paths)))

packages_userbin := $(shell bash scripts/chef.sh packages userbin)

kernel_destination := $(shell bash scripts/chef.sh config install-kernel)
bootloader_destination := $(shell bash scripts/chef.sh config install-bootloader)

ovmf_target := run/ovmf/vars.fd run/ovmf/code.fd

BOOT_IMG_SIZE_MB := 64
BOOT_PART_END_MIB := 65
DISK_IMG_SIZE_MB := 128

.PHONY: nothing
nothing:

.PHONY: clean
clean:
	rm -rf run
	cargo clean
