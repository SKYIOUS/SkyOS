"""Build the full SARGA OS bootable ISO in one command.

Orchestrates: kernel build → userspace build → initrd → builder → ISO.
Requires: Rust nightly, Python 3, xorriso (or WSL on Windows).
"""
import os, sys, subprocess, shutil

def run(cmd, cwd=None, desc=None):
    if desc:
        print(f"\n=== {desc} ===")
    print(f"$ {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        print(f"ERROR: command failed with code {result.returncode}")
        sys.exit(result.returncode)

def find_kernel_repo(root_dir):
    """Find SKYIOUS KERNEL repo (sibling dir or subdir)."""
    for name in ['SKYIOUS KERNEL', 'SKYIOUS-KERNEL']:
        # Sibling (local layout)
        p = os.path.join(root_dir, '..', name)
        if os.path.isdir(p):
            return p
        # Subdir (CI layout)
        p = os.path.join(root_dir, name)
        if os.path.isdir(p):
            return p
    return None

def main():
    root_dir = os.path.dirname(os.path.abspath(__file__))  # SkyOS root
    kernel_dir = find_kernel_repo(root_dir)

    if not kernel_dir:
        print("ERROR: SKYIOUS KERNEL repo not found (checked siblings and subdirs)")
        sys.exit(1)

    version = sys.argv[1] if len(sys.argv) > 1 else '0.6.0'
    profile = 'release'

    print(f"SkyOS root:  {root_dir}")
    print(f"Kernel repo: {kernel_dir}")
    print(f"Version:     {version}")
    print(f"Profile:     {profile}")

    # 1. Build kernel
    run(
        ['cargo', 'build', f'--{profile}', '--target', 'x86_64-unknown-none'],
        cwd=os.path.join(kernel_dir, 'kernel'),
        desc=f'Step 1/6: Build kernel ({profile})',
    )

    # 2. Build userspace
    run(
        ['cargo', 'build', '-Zbuild-std=core,alloc', '--target', 'x86_64-sarga.json', f'--{profile}'],
        cwd=root_dir,
        desc=f'Step 2/6: Build userspace ({profile})',
    )

    # 3. Build initrd
    run(
        ['python', 'build_initrd.py'],
        cwd=root_dir,
        desc='Step 3/6: Build initrd',
    )

    # 4. Copy initrd to builder fallback path
    initrd_src = os.path.join(root_dir, 'initrd.tar')
    initrd_dst_dir = os.path.join(kernel_dir, 'SkyOS')
    os.makedirs(initrd_dst_dir, exist_ok=True)
    shutil.copy2(initrd_src, os.path.join(initrd_dst_dir, 'initrd.tar'))
    print("  initrd copied to builder fallback path")

    # 5. Run builder (creates UEFI bootimage)
    run(
        ['cargo', 'run', f'--{profile}', '--manifest-path', os.path.join(kernel_dir, 'builder', 'Cargo.toml')],
        cwd=kernel_dir,
        desc='Step 4/6: Run builder (UEFI bootimage)',
    )

    # 6. Create ISO
    run(
        ['python', os.path.join(root_dir, 'scripts', 'make_iso.py'), version],
        cwd=root_dir,
        desc='Step 5/6: Create ISO',
    )

    # Summary
    iso_path = os.path.join(root_dir, 'release', f'skyos-{version}.iso')
    if os.path.exists(iso_path):
        size = os.path.getsize(iso_path)
        print(f"\n{'='*60}")
        print(f"SUCCESS: {iso_path} ({size/1024/1024:.1f} MB)")
        print(f"{'='*60}")
    else:
        print(f"\nWARNING: ISO not found at {iso_path}")

if __name__ == '__main__':
    main()
