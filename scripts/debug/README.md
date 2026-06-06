# Debug / Analysis Scripts — Usage Guide

All scripts in this directory analyze compiled ELF binaries in `target/x86_64-sarga/release/`.
Build with `./build.sh` first, then run any script.

| Script | What it checks | Use case |
|--------|---------------|----------|
| `check_elf.py` | ELF header, program headers, sections | Verify binary format, entry point, linking |
| `check_str.py` | Strings in .rodata at fixed offset | Quick check if config paths are embedded |
| `check_str2.py` | Strings per LOAD segment | Find which segment has a string |
| `check_str3.py` | Full binary string offsets | Find EVERY occurrence of a string |
| `check_str4.py` | Byte-level /etc/init.cfg search | Debug config path embedding issues |
| `check_str5.py` | .data hex dump + /etc/init search | Inspect writable segment contents |
| `check_init.py` | Full string table dump | See all printable strings >= 6 chars |

All scripts hardcode `target/x86_64-sarga/release/init` as the input binary.
