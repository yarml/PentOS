# $(BOOTLOADER_DEBUG): build-bootloader-debug
# $(BOOTLOADER_RELEASE): build-bootloader-release
# $(KERNEL_DEBUG): build-kernel-debug
# $(KERNEL_RELEASE): build-kernel-release

# define userbin_rule
# target/user/release/$(notdir $(1)): | build-userbin-$(1)-release
# target/user/debug/$(notdir $(1)): | build-userbin-$(1)-debug
# endef

# $(foreach p,$(packages_userbin),$(eval $(call userbin_rule,$(p))))
