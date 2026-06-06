# SkyOS Scripts Master List

## Build Scripts

### `SkyOS/Makefile`
Top-level GNU Makefile. Targets: `all` (build), `clean`, `run` (QEMU).
```
make [all|clean|run]
```

### `SkyOS/build.sh`
Builds SkyOS workspace (Linux/WSL). Compiles all workspace members for `x86_64-unknown-none`, then creates disk image.
```
./build.sh [all | component_name]
```

### `SkyOS/build.ps1`
PowerShell equivalent of `build.sh` (Windows). Calls WSL for `disk/create_disk.sh`.
```
.\build.ps1 [all | component_name]
```

### `SKYIOUS KERNEL/build_userspace.ps1`
Builds all userspace binaries (init, sargash, svc, vahid, sarga-disp, skypkg, login, passwd, skybuild, setup, coreutils, etc.) for the `x86_64-skyos` target. Copies to `SkyOS/bin/`, then creates `initrd.tar`.
```
.\build_userspace.ps1
```

### `SKYIOUS KERNEL/make_bootimage.ps1`
Full pipeline: builds userspace → builds kernel → runs Rust builder → produces UEFI bootimage.
```
.\make_bootimage.ps1
```

### `SKYIOUS KERNEL/make_bootimage.sh`
Bash equivalent of `make_bootimage.ps1` (Linux/WSL). Uses old `velox` naming.
```
./make_bootimage.sh
```

### `SKYIOUS KERNEL/build_disk.py`
Builds kernel, runs Rust builder, converts bootimage to VDI (VirtualBox) via VBoxManage.
```
python build_disk.py
```

### `SKYIOUS KERNEL/kernel/build.rs`
Cargo build script — monitors `linker.ld` and `initrd.tar` for changes, triggers kernel rebuild.
_(Automatic — no manual invocation needed.)_

### `SKYIOUS KERNEL/builder/src/main.rs`
Rust binary that wraps kernel ELF with UEFI bootloader (bootloader crate v0.11). Produces `bootimage-vahi_kernel.bin`.
_(Automatic — run via `cargo run --manifest-path builder/Cargo.toml`.)_

---

## Disk / Initrd Image Scripts

### `SkyOS/disk/create_disk.sh`
Creates 128 MB ext2 disk image (`aethos.img`), mounts via loopback, copies binaries and config. Requires root.
```
./disk/create_disk.sh
```

### `SKYIOUS KERNEL/build_initrd.py`
Creates `initrd.tar` with full FHS: ~40 userspace binaries, symlinks, config files, empty directories.
```
python build_initrd.py [SkyOS_directory]
```

### `SkyOS/scripts/make_sarga_image.py`
End-to-end image creator: stages binaries from `target/x86_64-sarga/release/`, packs initrd, embeds into kernel, converts to VDI.
```
python scripts/make_sarga_image.py
```

### `SkyOS/scripts/make_initrd.py`
Simple initrd packer from `staging/` directory.
```
python scripts/make_initrd.py
```

### `SKYIOUS KERNEL/rebuild_initrd.ps1`
Finds newly built `init` binary by size (18632 bytes), copies to `SkyOS/bin/init`, rebuilds initrd.
```
.\rebuild_initrd.ps1
```

---

## Run / QEMU Scripts

### `SkyOS/run.sh`
Launches SkyOS in QEMU (Linux/WSL). IDE drive, 512 MB, 2 CPUs, serial stdio, VGA.
```
./run.sh
```

### `SkyOS/run.ps1`
Launches SkyOS in QEMU (Windows). IDE drive, OVMF UEFI, 512 MB, 2 CPUs, serial stdio, VGA.
```
.\run.ps1
```

### `SKYIOUS KERNEL/run_qemu_display.ps1`
Launches SkyOS with SDL graphical display. Serial output logged to `qemu_display.log`. Uses 1 CPU, US keyboard, VGA std.
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

### `SkyOS/scripts/debug/check_elf.py`
Dumps ELF header and program headers of `target/x86_64-sarga/release/init`.
```
python scripts/debug/check_elf.py
```

### `SkyOS/scripts/debug/check_init.py`
Extracts all printable ASCII strings (≥6 chars) from the `init` binary with file offsets.
```
python scripts/debug/check_init.py
```

### `SkyOS/scripts/debug/check_str.py`
Searches segment 2 of `init` binary for strings `/etc/init.cfg`, `SkyOS`, `Starting`.
```
python scripts/debug/check_str.py
```

### `SkyOS/scripts/debug/check_str2.py`
Iterates LOAD segments, reports which contain key strings and virtual addresses.
```
python scripts/debug/check_str2.py
```

### `SkyOS/scripts/debug/check_str3.py`
Full binary scan for `/etc/init.cfg`, `ABCDEFGHIJKLMNOPQRSTUVWXYZ`, `OK`, `FAIL` — reports all file offsets.
```
python scripts/debug/check_str3.py
```

### `SkyOS/scripts/debug/check_str4.py`
Byte-level search for `/etc/init.cfg` variants in `init` binary. Hex dumps of `.rodata`.
```
python scripts/debug/check_str4.py
```

### `SkyOS/scripts/debug/check_str5.py`
Segment-aware search for `/etc/init` patterns. Hex dump of `.data` section.
```
python scripts/debug/check_str5.py
```

### `SKYIOUS KERNEL/check_init.py`
Extracts strings from `bin/init` and `bin/echo` inside `SkyOS/initrd.tar`.
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
Removes ALL build artifacts from both SkyOS and SKYIOUS KERNEL repos: target dirs, images, logs, binaries, initrd.
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
Syncs a single binary to `SkyOS/bin/` and rebuilds `initrd.tar`. Useful when iterating on one userspace component.
```
.\scripts\update_binaries.ps1 -Binary init
.\scripts\update_binaries.ps1 -Binary ls
```

---

## Installer ISO Scripts

### `scripts/build_installer_iso.py`
Creates a bootable SkyOS installer ISO (`skyos-installer.iso`) or disk image (`skyos-installer.img`). Builds the full bootimage, creates installer initrd with packages, and produces ISO using xorriso (preferred), mkisofs, or a raw disk image fallback.
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
