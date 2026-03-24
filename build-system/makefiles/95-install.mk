.PHONY: install
install: build-bootloader-release build-kernel-release
	sudo cp target/uefi/release/bootloader.efi $(bootloader_destination)
	sudo cp target/kernel/release/kernel $(kernel_destination)