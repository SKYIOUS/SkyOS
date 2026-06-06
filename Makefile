.PHONY: all clean run

all:
	./build.sh all

clean:
	cargo clean
	rm -f aethos.img

run: all
	qemu-system-x86_64 \
		-bios /usr/share/qemu/OVMF.fd \
		-drive format=raw,file=../SKYIOUS\ KERNEL/target/x86_64-velox/debug/bootimage-velox_kernel.bin \
		-drive id=aethos,file=aethos.img,if=none,format=raw \
		-device ahci,id=ahci0 \
		-device ide-hd,drive=aethos,bus=ahci0.0 \
		-m 512M -smp 2 -serial stdio \
		-device e1000,netdev=net0 -netdev user,id=net0 \
		-vga std -no-reboot
