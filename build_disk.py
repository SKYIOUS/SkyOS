import subprocess
import os
import shutil
import sys
import argparse
import tempfile

def run_command(command, cwd=None, env=None, check=True):
    print(f"Running: {' '.join(command)}")
    full_env = os.environ.copy()
    cargo_path = os.path.join(os.path.expanduser("~"), ".cargo", "bin")
    full_env["PATH"] = os.pathsep.join([cargo_path, full_env.get("PATH", "")])
    if env:
        full_env.update(env)
    result = subprocess.run(command, cwd=cwd, env=full_env, shell=True)
    if check and result.returncode != 0:
        print(f"Error: Command failed with return code {result.returncode}")
        sys.exit(1)
    return result.returncode == 0

def build_userspace(root_dir, release=False):
    """Build userspace components."""
    print("\n--- Building Userspace ---")
    target = "x86_64-sarga.json"
    if release:
        run_command(["cargo", "build", "--target", target, "--release"], cwd=root_dir)
    else:
        run_command(["cargo", "build", "--target", target], cwd=root_dir)
    print("Userspace build complete.")

def build_initrd(root_dir):
    """Build initrd.tar from userspace binaries."""
    print("\n--- Building Initrd ---")
    initrd_script = os.path.join(root_dir, "build_initrd.py")
    if not os.path.exists(initrd_script):
        print(f"Warning: {initrd_script} not found, skipping initrd")
        return None
    
    run_command([sys.executable, initrd_script, root_dir], cwd=root_dir)
    initrd_path = os.path.join(root_dir, "initrd.tar")
    if os.path.exists(initrd_path):
        print(f"Initrd created: {initrd_path}")
        return initrd_path
    return None

def build_kernel(kernel_dir):
    """Build the kernel."""
    print("\n--- Building Kernel ---")
    if not os.path.isdir(kernel_dir):
        print("ERROR: kernel/ directory not found at", kernel_dir)
        print("The kernel lives in a separate repo. Build it there or copy it here.")
        sys.exit(1)
    
    kernel_crate_dir = os.path.join(kernel_dir, "kernel")
    run_command(["cargo", "+nightly", "build"], cwd=kernel_crate_dir)
    print("Kernel build complete.")

def _ensure_nightly_cargo_wrapper():
    """Create a wrapper script that invokes nightly cargo for build.rs subprocesses."""
    nightly_bin = os.path.join(os.path.expanduser("~"), ".rustup", "toolchains",
                               "nightly-x86_64-pc-windows-msvc", "bin")
    nightly_cargo = os.path.join(nightly_bin, "cargo.exe")
    nightly_rustc = os.path.join(nightly_bin, "rustc.exe")
    wrapper_path = os.path.join(tempfile.gettempdir(), "opencode", "cargo-nightly-wrapper.cmd")
    os.makedirs(os.path.dirname(wrapper_path), exist_ok=True)
    with open(wrapper_path, "w") as f:
        f.write(f"""@echo off
set RUSTC={nightly_rustc}
set RUSTC_WORKSPACE_WRAPPER=
{nightly_cargo} %*
""")
    return wrapper_path

def build_bootimage(root_dir):
    """Create UEFI bootimage using the bootloader builder."""
    print("\n--- Creating Bootimage ---")
    kernel_dir = os.path.join(root_dir, "kernel")
    builder_dir = os.path.join(kernel_dir, "builder")
    
    # Build the builder with stable toolchain (build.rs uses nightly for UEFI)
    builder_env = {"RUST_BACKTRACE": "1", "CARGO_NIGHTLY": _ensure_nightly_cargo_wrapper()}
    run_command(["cargo", "+stable", "build"], cwd=builder_dir, env=builder_env)
    
    # Run the compiled builder binary
    builder_bin = os.path.join(builder_dir, "target", "debug", "builder.exe")
    if not os.path.exists(builder_bin):
        print(f"Error: Builder binary not found at {builder_bin}")
        sys.exit(1)
    run_command([builder_bin], cwd=builder_dir, env=builder_env)
    
    target_triple = os.environ.get("VAHI_TARGET_TRIPLE", "x86_64-vahi")
    uefi_path = os.path.join(kernel_dir, "target", target_triple, "debug", "bootimage-vahi_kernel.bin")
    
    if not os.path.exists(uefi_path):
        print(f"Error: Could not find UEFI image at {uefi_path}")
        sys.exit(1)
    
    output_uefi = os.path.join(root_dir, "skyos_uefi.img")
    shutil.copy2(uefi_path, output_uefi)
    print(f"Bootimage created: {output_uefi}")
    return output_uefi

def build_vdi(uefi_path, root_dir):
    """Convert UEFI image to VirtualBox VDI format."""
    print("\n--- Creating VDI ---")
    output_vdi = os.path.join(root_dir, "skyos.vdi")
    
    if os.path.exists(output_vdi):
        try:
            os.remove(output_vdi)
        except Exception as e:
            print(f"Warning: Could not remove old VDI (is VirtualBox running?): {e}")
    
    vbox_path = shutil.which("VBoxManage") or r"C:\Program Files\Oracle\VirtualBox\VBoxManage"
    try:
        run_command([vbox_path, "convertfromraw", uefi_path, output_vdi, "--format", "VDI"])
        print(f"VDI created: {output_vdi}")
        
        # Resize to 64MB
        print("Resizing VDI to 64MB...")
        r = subprocess.run([vbox_path, "modifymedium", "disk", output_vdi, "--resize", "64"])
        if r.returncode != 0:
            print(f"Warning: VDI resize returned {r.returncode} (non-fatal)")
        return output_vdi
    except Exception as e:
        print("Warning: VBoxManage conversion failed.")
        print(f"Error detail: {e}")
        return None

def build_iso(uefi_path, root_dir, version="0.6.0"):
    """Create bootable ISO from UEFI image."""
    print("\n--- Creating ISO ---")
    make_iso_script = os.path.join(root_dir, "scripts", "make_iso.py")
    if not os.path.exists(make_iso_script):
        print(f"Warning: {make_iso_script} not found, skipping ISO")
        return None
    
    run_command([sys.executable, make_iso_script, version], cwd=root_dir)
    release_dir = os.path.join(root_dir, "release")
    iso_path = os.path.join(release_dir, f"skyos-{version}.iso")
    if os.path.exists(iso_path):
        print(f"ISO created: {iso_path}")
        return iso_path
    return None

def main():
    parser = argparse.ArgumentParser(description="SkyOS Build System - Single entry point")
    parser.add_argument("--kernel-only", action="store_true", help="Only build kernel and bootimage")
    parser.add_argument("--userspace-only", action="store_true", help="Only build userspace")
    parser.add_argument("--no-vdi", action="store_true", help="Skip VDI creation")
    parser.add_argument("--iso", action="store_true", help="Create ISO image")
    parser.add_argument("--version", default="0.6.0", help="Version string for ISO")
    parser.add_argument("--release", action="store_true", help="Build in release mode")
    args = parser.parse_args()
    
    root_dir = os.path.dirname(os.path.abspath(__file__))
    kernel_dir = os.path.join(root_dir, "kernel")
    
    print("=== SkyOS Build System ===")
    print(f"Root: {root_dir}")
    print(f"Kernel: {kernel_dir}")
    
    # Build userspace (unless kernel-only)
    if not args.kernel_only:
        build_userspace(root_dir, release=args.release)
        initrd_path = build_initrd(root_dir)
        if initrd_path:
            # Copy initrd to kernel crate dir (builder looks for kernel/initrd.tar relative to root)
            kernel_crate_dir = os.path.join(kernel_dir, "kernel")
            os.makedirs(kernel_crate_dir, exist_ok=True)
            shutil.copy2(initrd_path, os.path.join(kernel_crate_dir, "initrd.tar"))
    
    # Build kernel (unless userspace-only)
    if not args.userspace_only:
        build_kernel(kernel_dir)
        uefi_path = build_bootimage(root_dir)
        
        # Create VDI (unless skipped)
        if not args.no_vdi:
            build_vdi(uefi_path, root_dir)
        
        # Create ISO (if requested)
        if args.iso:
            build_iso(uefi_path, root_dir, args.version)
    
    print("\n=== Build Complete ===")
    print("\nTo run with QEMU:")
    print(f'  qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2')
    print("\nFor VirtualBox: Use skyos.vdi with EFI enabled in System > Motherboard.")

if __name__ == "__main__":
    main()
