# SARGA OS Scripts Master List

## Build Scripts

### `SARGA OS/build_disk.py` (PRIMARY BUILD ENTRY POINT)
Single cross-platform build script that handles the entire build pipeline:
- Builds userspace binaries for x86_64-sarga target
- Creates initrd.tar with FHS directory structure
- Builds kernel with bootloader integration
- Creates UEFI bootimage (skyos_uefi.img)
- Optionally converts to VirtualBox VDI format
- Optionally creates bootable ISO image

```
python build_disk.py                          # Full build (userspace → kernel → UEFI image → VDI)
python build_disk.py --kernel-only            # Kernel + UEFI image only (faster for kernel dev)
python build_disk.py --userspace-only         # Userspace only
python build_disk.py --no-vdi                 # Skip VDI creation
python build_disk.py --iso --version 0.6.0    # Full build + ISO output
python build_disk.py --release                # Build in release mode
```

### `SARGA OS/build_initrd.py`
Creates initrd.tar from built userspace binaries. Called automatically by build_disk.py.
```
python build_initrd.py [root_dir]
```

### `SARGA OS/scripts/make_iso.py`
Creates UEFI-bootable ISOHybrid from UEFI bootimage. Called automatically by build_disk.py with --iso flag.
```
python scripts/make_iso.py [version]
```

### `SARGA OS/scripts/release_build.ps1`
Production release build with optimizations. Wraps build_disk.py with release flags.
```
.\scripts\release_build.ps1
```

### `SARGA OS/Makefile`
Legacy GNU Makefile. Targets: `all` (build), `clean`, `run` (QEMU).
```
make [all|clean|run]
```

### `SARGA OS/build.sh`
Legacy shell script. Use build_disk.py instead.
```
./build.sh [all | component_name]
```

### `SKYIOUS KERNEL/builder/src/main.rs`
Rust binary that wraps kernel ELF with UEFI bootloader (bootloader crate v0.11). Produces `bootimage-vahi_kernel.bin`.
_(Automatic — run via `cargo run --manifest-path builder/Cargo.toml`.)_

---

## Disk / Initrd Image Scripts

### `SARGA OS/disk/create_disk.sh`
Creates 128 MB ext2 disk image (`sarga.img`), mounts via loopback, copies binaries and config. Requires root.
```
./disk/create_disk.sh
```

### `SKYIOUS KERNEL/build_initrd.py`
Creates `initrd.tar` with full FHS: ~40 userspace binaries, symlinks, config files, empty directories.
```
python build_initrd.py [SARGA-OS_directory]
```

### `SARGA OS/scripts/make_sarga_image.py`
End-to-end image creator: stages binaries from `target/x86_64-sarga/release/`, packs initrd, embeds into kernel, converts to VDI.
```
python scripts/make_sarga_image.py
```

### `SARGA OS/scripts/make_initrd.py`
Simple initrd packer from `staging/` directory.
```
python scripts/make_initrd.py
```

### `SKYIOUS KERNEL/rebuild_initrd.ps1`
Finds newly built `init` binary by size (18632 bytes), copies to `SARGA OS/bin/init`, rebuilds initrd.
```
.\rebuild_initrd.ps1
```

---

## Run / QEMU Scripts

### `SARGA OS/run.sh`
Launches SARGA OS in QEMU (Linux/WSL). IDE drive, 512 MB, 2 CPUs, serial stdio, VGA.
```
./run.sh
```

### `SARGA OS/run.ps1`
Launches SARGA OS in QEMU (Windows). IDE drive, OVMF UEFI, 512 MB, 2 CPUs, serial stdio, VGA.
```
.\run.ps1
```

### `SKYIOUS KERNEL/run_qemu_display.ps1`
Launches SARGA OS with SDL graphical display. Serial output logged to `qemu_display.log`. Uses 1 CPU, US keyboard, VGA std.
```
.\run_qemu_display.ps1
```

### `SKYIOUS KERNEL/run_test_nographic.ps1`
Headless boot test — runs QEMU nographic, waits 30s, checks for `login:` prompt. Exit 0 = pass, 1 = fail.
```
.\run_test_nographic.ps1
```

---

## Test Scripts

### `SKYIOUS KERNEL/tests/test_boot.ps1`
Boot sanity test — launches QEMU nographic, captures serial output, passes if `login:` is found.
```
.\tests\test_boot.ps1
```

### `SKYIOUS KERNEL/tests/test_panic.ps1`
Panic recovery test — launches QEMU with `-append "panic=1"`, passes if output contains `PANIC`.
```
.\tests\test_panic.ps1
```

### `SKYIOUS KERNEL/tests/test_login.ps1`
Login automation test — uses Expect to send username/password (`root`/`root`), passes on shell prompt `$`.
```
.\tests\test_login.ps1
```

---

## Debug / Analysis Scripts

### `SARGA OS/scripts/debug/check_elf.py`
Dumps ELF header and program headers of `target/x86_64-sarga/release/init`.
```
python scripts/debug/check_elf.py
```

### `SARGA OS/scripts/debug/check_init.py`
Extracts all printable ASCII strings (≥6 chars) from the `init` binary with file offsets.
```
python scripts/debug/check_init.py
```

### `SARGA OS/scripts/debug/check_str.py`
Searches segment 2 of `init` binary for strings `/etc/init.cfg`, `SARGA OS`, `Starting`.
```
python scripts/debug/check_str.py
```

### `SARGA OS/scripts/debug/check_str2.py`
Iterates LOAD segments, reports which contain key strings and virtual addresses.
```
python scripts/debug/check_str2.py
```

### `SARGA OS/scripts/debug/check_str3.py`
Full binary scan for `/etc/init.cfg`, `ABCDEFGHIJKLMNOPQRSTUVWXYZ`, `OK`, `FAIL` — reports all file offsets.
```
python scripts/debug/check_str3.py
```

### `SARGA OS/scripts/debug/check_str4.py`
Byte-level search for `/etc/init.cfg` variants in `init` binary. Hex dumps of `.rodata`.
```
python scripts/debug/check_str4.py
```

### `SARGA OS/scripts/debug/check_str5.py`
Segment-aware search for `/etc/init` patterns. Hex dump of `.data` section.
```
python scripts/debug/check_str5.py
```

### `SKYIOUS KERNEL/check_init.py`
Extracts strings from `bin/init` and `bin/echo` inside `SARGA OS/initrd.tar`.
```
python check_init.py
```

---

## Dev Environment Scripts

### `scripts/setup_dev.ps1`
One-click Windows dev environment setup: checks Rust, installs nightly, targets, bootimage, QEMU, Python, OVMF.
```
.\scripts\setup_dev.ps1 [-Force]
```

### `scripts/setup_dev.sh`
One-click Linux/WSL dev environment setup (equivalent to the .ps1).
```
./scripts/setup_dev.sh
```

### `scripts/dev_loop.ps1`
Fast iterative development loop: builds userspace → kernel → bootimage → runs QEMU. Ideal for code → test → fix cycles.
```
.\scripts\dev_loop.ps1 [-Display] [-Timeout 30]
```

---

## Cleanup Scripts

### `scripts/clean_all.ps1`
Removes ALL build artifacts from both SARGA OS and SKYIOUS KERNEL repos: target dirs, images, logs, binaries, initrd.
```
.\scripts\clean_all.ps1
```

---

## Release Scripts

### `scripts/release_build.ps1`
Production release build with optimizations. Builds userspace + kernel in release mode, packages bootimage + archive into timestamped directory.
```
.\scripts\release_build.ps1
```

---

## Analysis / Profiling Scripts

### `scripts/analyze_kernel_size.py`
Break down kernel ELF binary by section (.text, .rodata, .data, .bss) with sizes and percentages.
```
python scripts/analyze_kernel_size.py [path_to_elf]
```

### `scripts/size_report.py`
Tracks kernel + userspace binary sizes over time. Stores snapshots in `size_history.json`. Shows size changes vs previous and first measurement.
```
python scripts/size_report.py
```

### `scripts/parse_qemu_log.py`
Extracts structured info from QEMU serial logs: tag frequency, panic messages, boot timeline, last lines.
```
python scripts/parse_qemu_log.py [logfile]
```

---

## Test Scripts

### `scripts/test_all.ps1`
Discovers and runs all test scripts from the `tests/` directory. Reports pass/fail count per test.
```
.\scripts\test_all.ps1
```

### `scripts/run_gdb.ps1`
Launches QEMU with GDB server on port 1234, halts at boot, and attaches GDB/LLDB for kernel debugging.
```
.\scripts\run_gdb.ps1 [-NoGdb] [-Port 1234]
```

---

## Utility Scripts

### `scripts/update_binaries.ps1`
Syncs a single binary to `SARGA OS/bin/` and rebuilds `initrd.tar`. Useful when iterating on one userspace component.
```
.\scripts\update_binaries.ps1 -Binary init
.\scripts\update_binaries.ps1 -Binary ls
```

---

## Installer ISO Scripts

### `scripts/build_installer_iso.py`
Creates a bootable SARGA OS installer ISO (`sarga-installer.iso`) or disk image (`sarga-installer.img`). Builds the full bootimage, creates installer initrd with packages, and produces ISO using xorriso (preferred), mkisofs, or a raw disk image fallback.
```
python scripts/build_installer_iso.py           # Use existing bootimage
python scripts/build_installer_iso.py --full    # Full rebuild + ISO
```

### `scripts/build_installer_iso.sh`
Bash equivalent of `build_installer_iso.py` (Linux/WSL). Automatically detects xorriso, mkisofs, or falls back to raw disk image.
```
./scripts/build_installer_iso.sh
./scripts/build_installer_iso.sh --full
```
