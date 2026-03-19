.PHONY: flat-boot-debug flat-boot-release
flat-boot-debug: build-bootloader-debug build-kernel-debug
	mkdir -p run/debug/flat/boot/efi/boot
	cp target/uefi/debug/bootloader.efi run/debug/flat/boot/efi/boot/bootx64.efi
	cp target/kernel/debug/kernel run/debug/flat/boot/pentos.kernel

flat-boot-release: build-bootloader-release build-kernel-release
	mkdir -p run/release/flat/boot/efi/boot
	cp target/uefi/release/bootloader.efi run/release/flat/boot/efi/boot/bootx64.efi
	cp target/kernel/release/kernel run/release/flat/boot/pentos.kernel

.PHONY: flat-main-debug flat-main-release
flat-main-debug: build-kernel-debug
	mkdir -p run/debug/flat/main/sys
	mkdir -p run/debug/flat/main/pkg/bin
	cp target/kernel/debug/kernel run/debug/flat/main/sys/kernel

flat-main-release: build-kernel-release
	mkdir -p run/release/flat/main/sys
	mkdir -p run/release/flat/main/pkg/bin
	cp target/kernel/release/kernel run/release/flat/main/sys/kernel

.PHONY: flat-debug flat-release
flat-debug: flat-boot-debug flat-main-debug
flat-release: flat-boot-release flat-main-release
