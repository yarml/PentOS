FLAT_BOOT_RELEASE_FILES := run/release/flat/boot/efi/boot/bootx64.efi run/release/flat/boot/pentos.kernel
FLAT_BOOT_DEBUG_FILES := run/debug/flat/boot/efi/boot/bootx64.efi run/debug/flat/boot/pentos.kernel

FLAT_MAIN_RELEASE_FILES := run/release/flat/main/sys/kernel $(foreach p,$(packages_userbin),run/release/flat/main/pkg/$(dir $(p))bin/$(notdir $(p)))
FLAT_MAIN_DEBUG_FILES := run/debug/flat/main/sys/kernel $(foreach p,$(packages_userbin),run/debug/flat/main/pkg/$(dir $(p))bin/$(notdir $(p)))

run/release/flat/boot/efi/boot/bootx64.efi: | build-bootloader-release
run/release/flat/boot/efi/boot/bootx64.efi: $(BOOTLOADER_RELEASE)
	mkdir -p $(dir $@)
	cp $< $@

run/release/flat/boot/pentos.kernel: | build-kernel-release
run/release/flat/boot/pentos.kernel: $(KERNEL_RELEASE)
	mkdir -p $(dir $@)
	cp $< $@

run/debug/flat/boot/efi/boot/bootx64.efi: | build-bootloader-debug
run/debug/flat/boot/efi/boot/bootx64.efi: $(BOOTLOADER_DEBUG)
	mkdir -p $(dir $@)
	cp $< $@

run/debug/flat/boot/pentos.kernel: | build-kernel-debug
run/debug/flat/boot/pentos.kernel: $(KERNEL_DEBUG)
	mkdir -p $(dir $@)
	cp $< $@

run/release/flat/main/sys/kernel: | build-kernel-release
run/release/flat/main/sys/kernel: $(KERNEL_RELEASE)
	mkdir -p $(dir $@)
	cp $< $@

run/debug/flat/main/sys/kernel: | build-kernel-debug
run/debug/flat/main/sys/kernel: $(KERNEL_DEBUG)
	mkdir -p $(dir $@)
	cp $< $@

define userbin_flat_rule
run/release/flat/main/pkg/$(dir $(1))bin/$(notdir $(1)): build-userbin-$(1)-release
run/release/flat/main/pkg/$(dir $(1))bin/$(notdir $(1)): target/user/release/$(notdir $(1))
	mkdir -p $$(dir $$@)
	cp $$< $$@

run/debug/flat/main/pkg/$(dir $(1))bin/$(notdir $(1)): build-userbin-$(1)-debug
run/debug/flat/main/pkg/$(dir $(1))bin/$(notdir $(1)): target/user/debug/$(notdir $(1))
	mkdir -p $$(dir $$@)
	cp $$< $$@
endef

$(foreach p,$(packages_userbin),$(eval $(call userbin_flat_rule,$(p))))

.PHONY: flat-boot-release flat-boot-debug flat-main-release flat-main-debug
.PHONY: flat-release flat-debug img-boot-release img-boot-debug

flat-boot-release: $(FLAT_BOOT_RELEASE_FILES)
flat-boot-debug: $(FLAT_BOOT_DEBUG_FILES)
flat-main-release: $(FLAT_MAIN_RELEASE_FILES)
flat-main-debug: $(FLAT_MAIN_DEBUG_FILES)
flat-release: flat-boot-release flat-main-release
flat-debug: flat-boot-debug flat-main-debug
