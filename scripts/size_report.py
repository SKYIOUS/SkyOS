"""size_report.py — Track kernel and userspace binary sizes over time."""

import datetime
import json
import os

REPORT_FILE = "size_history.json"
PATHS = [
    (
        "Kernel (debug)",
        "../SKYIOUS KERNEL/kernel/target/x86_64-unknown-none/debug/vahi_kernel",
    ),
    (
        "Kernel (release)",
        "../SKYIOUS KERNEL/kernel/target/x86_64-unknown-none/release/vahi_kernel",
    ),
    (
        "Bootimage (debug)",
        "../SKYIOUS KERNEL/target/x86_64-vahi/debug/bootimage-vahi_kernel.bin",
    ),
    (
        "Bootimage (release)",
        "../SKYIOUS KERNEL/target/x86_64-vahi/release/bootimage-vahi_kernel.bin",
    ),
    ("initrd.tar", "../SKYIOUS KERNEL/SkyOS/initrd.tar"),
    ("Init binary", "target/x86_64-sarga/release/init"),
]


def get_sizes():
    sizes = {}
    for name, path in PATHS:
        full = os.path.join(os.path.dirname(__file__) or ".", path)
        if os.path.exists(full):
            sizes[name] = os.path.getsize(full)
    return sizes


def load_history():
    if os.path.exists(REPORT_FILE):
        with open(REPORT_FILE) as f:
            return json.load(f)
    return []


def save_history(history):
    with open(REPORT_FILE, "w") as f:
        json.dump(history, f, indent=2)


def print_table(entry):
    print(f"{'Component':<35} {'Size':>10} {'vs Prev':>10} {'vs First':>10}")
    print("-" * 70)
    first = history[0] if len(history) > 1 else entry
    prev = history[-2] if len(history) >= 2 else entry
    for name in sorted(entry["sizes"]):
        sz = entry["sizes"][name]
        psz = prev["sizes"].get(name, 0)
        fsz = first["sizes"].get(name, 0)
        pdiff = sz - psz
        fdiff = sz - fsz
        pp = f"{'+' if pdiff > 0 else ''}{pdiff:>9,}" if psz else "       -"
        fp = f"{'+' if fdiff > 0 else ''}{fdiff:>9,}" if fsz else "       -"
        print(f"{name:<35} {sz:>10,} {pp:>10} {fp:>10}")


if __name__ == "__main__":
    history = load_history()
    sizes = get_sizes()
    if not sizes:
        print("No binaries found. Build the project first.")
        exit(1)
    entry = {"date": datetime.datetime.now().isoformat(), "sizes": sizes}
    # Only append if sizes differ from last entry
    if not history or history[-1]["sizes"] != sizes:
        history.append(entry)
        save_history(history)
        print("Added new size snapshot.\n")
    else:
        print("Sizes unchanged.\n")
    print_table(entry)
    print(f"\nSnapshots: {len(history)}")
