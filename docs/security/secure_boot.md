# Secure Boot

SkyOS does not currently implement Secure Boot, kernel image signing, or TPM measured boot. The kernel is built as a UEFI boot image (`bootloader` crate → `kernel/builder`) and booted directly by OVMF/firmware with no signature verification.

## Current State

- **Boot**: UEFI firmware → `kernel/builder` boot image (`bootimage-vahi_kernel.bin`) → kernel. No signature checks.
- **Signing tooling**: No `scripts/sign-efi.py` or signing step exists in the build.
- **TPM**: No TPM driver or PCR measurement code exists in the kernel.

## Planned

If Secure Boot support is added, the chain would be:

1. UEFI firmware verifies the boot image signature (requires enrolling a kernel signing key in firmware, or MOK/Shim).
2. Kernel verifies the initramfs (`initrd.tar`) signature before unpacking.
3. Runtime integrity monitoring (far future).

## Enabling UEFI Secure Boot Today

Not supported — the boot image is unsigned. Do not enable UEFI Secure Boot in firmware or the kernel will not boot.
