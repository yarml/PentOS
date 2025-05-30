ifeq ("$(wildcard target/debug/chef)","")
$(info Compiling chef: May take a while)
endif

.PHONY: nothing
nothing:

packages = $(shell cargo chef packages)

kernel_destination = $(shell cargo chef config install-kernel)
bootloader_destination = $(shell cargo chef config install-bootloader)

ovmf_target = run/ovmf/vars.fd run/ovmf/code.fd

.PHONY: check-all clippy doc
define package_build_recipe =
.PHONY: build-$(1) check-$(1) clippy-$(1) doc-$(1)
build-$(1):
	cd $(1) && cargo build -p $(1)
check-all: check-$(1)
check-$(1):
	@cd $(1) && cargo clippy --all-features --keep-going --quiet --message-format=json -p $(1)
clippy: clippy-$(1)
clippy-$(1):
	cd $(1) && cargo clippy --no-deps --all-features --keep-going -p $(1)
doc: doc-$(1)
doc-$(1):
	cd $(1) && cargo doc --no-deps --all-features -p $(1)
endef

$(foreach package,$(packages),$(eval $(call package_build_recipe,$(package))))

$(ovmf_target):
	cargo chef ovmf

.PHONY: image
image: $(ovmf_target) build-bootloader build-kernel
	mkdir -p run/esp/efi/boot
	cp target/uefi/debug/bootloader.efi run/esp/efi/boot/bootx64.efi
	cp target/kernel/debug/kernel run/esp/pentos.kernel

.PHONY: run
run: image
	bash scripts/debug.sh

.PHONY: install
install: build-bootloader build-kernel
	sudo cp target/uefi/debug/bootloader.efi $(bootloader_destination)
	sudo cp target/kernel/debug/kernel $(kernel_destination)

.PHONY: test
test:
	cargo test --workspace --no-fail-fast --exclude kernel --exclude bootloader
