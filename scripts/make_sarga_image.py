import os
import shutil
import subprocess
import sys

# ── Paths ────────────────────────────────────────────────────────────────────
ROOT_DIR     = r"c:\Users\nanda\Desktop\Github\SkyOS"
KERNEL_DIR   = r"c:\Users\nanda\Desktop\Github\SKYIOUS KERNEL"
KERNEL_IMAGE = os.path.join(KERNEL_DIR, r"target\x86_64-vahi\debug\bootimage-vahi_kernel.bin")
TARGET_DIR   = os.path.join(ROOT_DIR, "target", "x86_64-sarga", "release")
VBOX         = r"C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"

# ── Binaries to include in initrd ────────────────────────────────────────────
BINARIES = {
    "init":  "init",
    "sash":  "sash",
    "proc":  "proc",
    "cat":   "coreutils",
    "ls":    "coreutils",
    "mkdir": "coreutils",
    "rm":    "coreutils",
    "echo":  "coreutils",
    "uname": "coreutils",
    "true":  "coreutils",
    "false": "coreutils",
}

def run(cmd, cwd=None, fatal=True):
    print(f"  > {' '.join(cmd)}")
    res = subprocess.run(cmd, cwd=cwd, shell=True)
    if res.returncode != 0 and fatal:
        print(f"  [ERROR] Command failed (exit {res.returncode})")
        sys.exit(1)
    return res.returncode

def main():
    print("=" * 60)
    print("  Sarga OS Image Creator")
    print("=" * 60)

    # ── Step 1: Stage the root filesystem ────────────────────────────────────
    print("\n[1/5] Staging root filesystem...")
    staging = os.path.join(ROOT_DIR, "staging")
    if os.path.exists(staging):
        shutil.rmtree(staging)
    for d in ["bin", "proc", "etc", "tmp", "dev", os.path.join("usr", "bin")]:
        os.makedirs(os.path.join(staging, d))

    for bin_name in BINARIES:
        src = os.path.join(TARGET_DIR, bin_name)
        if not os.path.exists(src):
            print(f"  [WARN] {bin_name} not found, skipping.")
            continue
        shutil.copy2(src, os.path.join(staging, "bin", bin_name))
        print(f"  Copied {bin_name}")

    init_cfg = os.path.join(staging, "etc", "init.cfg")
    with open(init_cfg, "w") as f:
        f.write("# SkyOS init configuration\n")
        f.write("login /bin/sash\n")
    print("  Created /etc/init.cfg")

    # ── Step 2: Pack initrd.tar ───────────────────────────────────────────────
    print("\n[2/5] Creating initrd.tar...")
    initrd = os.path.join(ROOT_DIR, "initrd.tar")
    run(["tar", "-cvf", initrd, "-C", staging, "."])
    size_mb = os.path.getsize(initrd) / 1024 / 1024
    print(f"  initrd.tar created ({size_mb:.1f} MB)")

    # Copy initrd to kernel's SkyOS/ dir so include_bytes! picks it up
    kernel_initrd = os.path.join(KERNEL_DIR, "SkyOS", "initrd.tar")
    shutil.copy2(initrd, kernel_initrd)
    print(f"  Copied to kernel SkyOS/ ({os.path.getsize(kernel_initrd)} bytes)")

    # ── Step 3: Rebuild kernel (embeds initrd via include_bytes!) ────────────
    print("\n[3/5] Rebuilding Vahi Kernel (embedding initrd)...")
    ret = run(["cargo", "run", "--manifest-path", "builder\\Cargo.toml"],
              cwd=KERNEL_DIR, fatal=False)
    if ret != 0:
        print("  [WARN] Builder failed — using existing kernel binary.")

    if not os.path.exists(KERNEL_IMAGE):
        print(f"  [ERROR] Kernel image not found at {KERNEL_IMAGE}")
        sys.exit(1)

    img_size_mb = os.path.getsize(KERNEL_IMAGE) / 1024 / 1024
    print(f"  Kernel binary: {img_size_mb:.1f} MB")

    # ── Step 4: Convert to VDI ───────────────────────────────────────────────
    print("\n[4/5] Creating VirtualBox VDI...")
    output_vdi = os.path.join(ROOT_DIR, "sarga.vdi")
    if os.path.exists(output_vdi):
        try: os.remove(output_vdi)
        except OSError: pass

    if os.path.exists(VBOX):
        run([VBOX, "convertfromraw", KERNEL_IMAGE, output_vdi, "--format", "VDI"])
        vdi_mb = os.path.exists(output_vdi) and os.path.getsize(output_vdi) / 1024 / 1024
        print(f"  sarga.vdi created ({vdi_mb:.1f} MB)" if vdi_mb else "  [WARN] VDI may not have been created.")
    else:
        print("  [WARN] VBoxManage not found — skipping VDI creation.")

    # ── Step 5: Summary ──────────────────────────────────────────────────────
    print("\n[5/5] Done!")
    print(f"\n  Kernel : {KERNEL_IMAGE}")
    print(f"  initrd : {initrd}  (embedded in kernel)")
    print(f"  VDI    : {output_vdi}")
    print()
    print("  ┌─ QEMU (single drive) ─────────────────────────────────────────")
    print(f"  │  qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=\"{KERNEL_IMAGE}\" -m 512M -smp 2 -serial stdio -vga std")
    print("  │")
    print("  └─ VirtualBox ──────────────────────────────────────────────────")
    print(f"     1. Create VM → Other Linux (64-bit)")
    print( "     2. System → Motherboard → ☑ Enable EFI")
    print(f"     3. Storage → Add {output_vdi} as primary disk")
    print( "     4. Boot! (initrd is embedded in the kernel)")

if __name__ == "__main__":
    main()
