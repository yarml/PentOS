.PHONY: vmdk-release vmdk-debug
vmdk-release: img-release
	rm -f run/release/pentos.vmdk
	VBoxManage convertfromraw run/release/pentos.img run/release/pentos.vmdk --format VMDK

vmdk-debug: img-debug
	rm -f run/debug/pentos.vmdk
	VBoxManage convertfromraw run/debug/pentos.img run/debug/pentos.vmdk --format VMDK
