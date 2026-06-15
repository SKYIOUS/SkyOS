import os
import shutil
import subprocess
import sys

# ── Paths ────────────────────────────────────────────────────────────────────
SCRIPT_DIR  = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR    = os.path.dirname(SCRIPT_DIR)
KERNEL_DIR  = os.path.normpath(os.path.join(ROOT_DIR, "..", "SKYIOUS KERNEL"))

# Determine profile (--release flag)
PROFILE = "release" if "--release" in sys.argv else "debug"

KERNEL_IMAGE = os.path.join(KERNEL_DIR, f"target\\x86_64-vahi\\{PROFILE}\\bootimage-vahi_kernel.bin")
TARGET_DIR   = os.path.join(ROOT_DIR, "target", "x86_64-sarga", PROFILE)
VBOX         = r"C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"

# ── Binaries to include in initrd ────────────────────────────────────────────
BINARIES = {
    # Core
    "init":  "init",
    "sash":  "sash",
    "proc":  "proc",
    # Coreutils
    "cat":   "coreutils",
    "ls":    "coreutils",
    "mkdir": "coreutils",
    "rm":    "coreutils",
    "echo":  "coreutils",
    "uname": "coreutils",
    "true":  "coreutils",
    "false": "coreutils",
    "df":    "coreutils",
    "ps":    "coreutils",
    "top":   "coreutils",
    "cp":    "coreutils",
    "mv":    "coreutils",
    "grep":  "coreutils",
    "head":  "coreutils",
    "wc":    "coreutils",
    "sort":  "coreutils",
    "find":  "coreutils",
    "kill":  "coreutils",
    "sleep": "coreutils",
    "chmod": "coreutils",
    "chown": "coreutils",
    "ln":    "coreutils",
    "readlink": "coreutils",
    "tee":   "coreutils",
    "which": "coreutils",
    "xargs": "coreutils",
    "date":  "coreutils",
    "hostname": "coreutils",
    "id":    "coreutils",
    "whoami": "coreutils",
    "uptime": "coreutils",
    "free":  "coreutils",
    "dd":    "coreutils",
    "sync":  "coreutils",
    "hexdump": "coreutils",
    "od":    "coreutils",
    "basename": "coreutils",
    "dirname": "coreutils",
    "passwd": "coreutils",
    "login": "coreutils",
    "su":    "coreutils",
    "env":   "coreutils",
    "ping":  "coreutils",
    "lspci": "coreutils",
    "mkfs_skyfs": "coreutils",
    "cut":   "coreutils",
    "tr":    "coreutils",
    "uniq":  "coreutils",
    "diff":  "coreutils",
    "tac":   "coreutils",
    "nl":    "coreutils",
    "sed":   "coreutils",
    "awk":   "coreutils",
    "patch": "coreutils",
    "tar":   "coreutils",
    "gzip":  "coreutils",
    "stat":  "coreutils",
    "touch": "coreutils",
    "du":    "coreutils",
    # Apps
    "sarga-term": "sarga-term",
    "ade":        "ade",
    "skyedit":    "skyedit",
    "calculator": "calculator",
    "skyfiles":   "skyfiles",
    "skysettings": "skysettings",
    "skyd":        "skyd",
    "skyview":     "skyview",
    # Nettools
    "curl":     "nettools",
    "nc":       "nettools",
    "echod":    "nettools",
    "ifconfig": "nettools",
    "resolve":  "nettools",
    # Package manager
    "spkg":  "spkg",
    # AI CLI
    "aicli": "aicli",
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
    print(f"  SkyOS Image Creator  [{PROFILE.upper()}]")
    print("=" * 60)

    # ── Step 1: Build userspace ────────────────────────────────────────────
    print("\n[1/6] Building userspace...")
    build_args = ["cargo", "build"]
    if PROFILE == "release":
        build_args.append("--release")
    run(build_args, cwd=ROOT_DIR, fatal=False)

    # ── Step 2: Stage the root filesystem ──────────────────────────────────
    print("\n[2/6] Staging root filesystem...")
    staging = os.path.join(ROOT_DIR, "staging")
    if os.path.exists(staging):
        shutil.rmtree(staging)
    for d in ["bin", "proc", "etc", "tmp", "dev", os.path.join("usr", "bin"), os.path.join("usr", "share", "fonts")]:
        os.makedirs(os.path.join(staging, d))

    copied = 0
    skipped = 0
    for bin_name in BINARIES:
        src = os.path.join(TARGET_DIR, bin_name)
        if not os.path.exists(src):
            print(f"  [WARN] {bin_name} not found, skipping.")
            skipped += 1
            continue
        shutil.copy2(src, os.path.join(staging, "bin", bin_name))
        copied += 1
    print(f"  Copied {copied} binaries ({skipped} skipped)")

    # Copy system fonts
    font_src = os.path.join(ROOT_DIR, "fonts", "DejaVuSans.ttf")
    if os.path.exists(font_src):
        font_dst = os.path.join(staging, "usr", "share", "fonts", "DejaVuSans.ttf")
        os.makedirs(os.path.dirname(font_dst), exist_ok=True)
        shutil.copy2(font_src, font_dst)
        print(f"  Copied DejaVuSans.ttf ({os.path.getsize(font_dst)} bytes)")
    else:
        print("  [WARN] DejaVuSans.ttf not found in fonts/")

    init_cfg = os.path.join(staging, "etc", "init.toml")
    with open(init_cfg, "w") as f:
        f.write("# SkyOS init configuration\n")
        f.write("hostname = \"sarga-os\"\n\n")
        f.write("[[service]]\n")
        f.write('name = "login"\n')
        f.write('exec = "/bin/sash"\n')
        f.write("respawn = true\n\n")
        f.write("[[service]]\n")
        f.write('name = "desktop"\n')
        f.write('exec = "/bin/ade"\n')
        f.write("respawn = true\n\n")
        f.write("[[service]]\n")
        f.write('name = "proc"\n')
        f.write('exec = "/bin/proc"\n')
        f.write("respawn = true\n")
    print("  Created /etc/init.toml")

    # ── Step 3: Pack initrd.tar ────────────────────────────────────────────
    print("\n[3/6] Creating initrd.tar...")
    initrd = os.path.join(ROOT_DIR, "initrd.tar")
    run(["tar", "-cvf", initrd, "-C", staging, "."])
    size_mb = os.path.getsize(initrd) / 1024 / 1024
    print(f"  initrd.tar created ({size_mb:.1f} MB)")

    # Copy initrd to kernel's SkyOS/ dir so include_bytes! picks it up
    kernel_initrd = os.path.join(KERNEL_DIR, "SkyOS", "initrd.tar")
    shutil.copy2(initrd, kernel_initrd)
    print(f"  Copied to kernel SkyOS/ ({os.path.getsize(kernel_initrd)} bytes)")

    # ── Step 4: Rebuild kernel ─────────────────────────────────────────────
    print(f"\n[4/6] Rebuilding Vahi Kernel [{PROFILE.upper()}]...")
    builder_args = ["cargo", "run", "--manifest-path", "builder\\Cargo.toml"]
    ret = run(builder_args, cwd=KERNEL_DIR, fatal=False)
    if ret != 0:
        print("  [WARN] Builder failed — using existing kernel binary.")

    if not os.path.exists(KERNEL_IMAGE):
        print(f"  [ERROR] Kernel image not found at {KERNEL_IMAGE}")
        sys.exit(1)

    img_size_mb = os.path.getsize(KERNEL_IMAGE) / 1024 / 1024
    print(f"  Kernel binary: {img_size_mb:.1f} MB")

    # ── Step 5: Convert to VDI ─────────────────────────────────────────────
    print("\n[5/6] Creating VirtualBox VDI...")
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

    # ── Step 6: Summary ────────────────────────────────────────────────────
    print("\n[6/6] Done!")
    print(f"\n  Profile: {PROFILE}")
    print(f"  Kernel : {KERNEL_IMAGE}")
    print(f"  initrd : {initrd}  (embedded in kernel)")
    print(f"  VDI    : {output_vdi}")
    print()
    print("  +-- QEMU -----------------------------------------------------------")
    print(f"  |  qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=\"{KERNEL_IMAGE}\" -m 2G -smp 1 -serial stdio -display sdl")
    print("  |")
    print("  +-- VirtualBox ------------------------------------------------------")
    print("     1. Create VM -> Other Linux (64-bit)")
    print("     2. System -> Motherboard -> Enable EFI")
    print(f"     3. Storage -> Add {output_vdi} as primary disk")
    print("     4. Boot! (initrd is embedded in the kernel)")

if __name__ == "__main__":
    main()
