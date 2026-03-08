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
