# Aethos OS

Aethos OS is a GPL-free userland operating system designed to run on top of the Velox Kernel. It is lightweight, fast, and uses a native companion language, Korlang.

## Building
1. Run `./build.sh all` to build everything.
2. The disk image `aethos.img` will be generated using `disk/create_disk.sh`.
3. To run in QEMU alongside Velox Kernel, use `make run`.

## License
MIT License. See LICENSE file for details.
