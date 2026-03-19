.PHONY: install
install: build-release-bootloader build-release-kernel
	sudo cp target/uefi/release/bootloader.efi $(bootloader_destination)
	sudo cp target/kernel/release/kernel $(kernel_destination)