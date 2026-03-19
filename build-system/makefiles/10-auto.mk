.PHONY: check-all clippy
define package_build_recipe =
.PHONY: build-$(1)-release build-$(1)-debug check-$(1) clippy-$(1) doc-$(1)
build-$(1)-debug:
	cd $(2) && cargo build -p $(1)
build-$(1)-release:
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

.PHONY: build-userbin-release build-userbin-debug
define package_userbin_recipe =
.PHONY: build-userbin-$(1)-release build-userbin-$(1)-debug

build-userbin-$(1)-release: build-$(notdir $(1))-release
build-userbin-$(1)-debug: build-$(notdir $(1))-debug

build-userbin-release: build-userbin-$(1)-release
build-userbin-debug: build-userbin-$(1)-debug
endef

$(foreach package,$(packages_userbin), \
	$(eval $(call \
		package_userbin_recipe,$(package) \
	)) \
)
