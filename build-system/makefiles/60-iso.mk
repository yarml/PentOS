.PHONY: iso-release iso-debug
iso-release: img-release
	rm -f run/release/pentos.iso
	VBoxManage convertfromraw run/release/pentos.img run/release/pentos.iso --format RAW

iso-debug: img-debug
	rm -f run/debug/pentos.iso
	VBoxManage convertfromraw run/debug/pentos.img run/debug/pentos.iso --format RAW
