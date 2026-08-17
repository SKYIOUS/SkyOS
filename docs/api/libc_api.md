# Userspace API Reference (`libsarga`)

SkyOS userspace is written in Rust (`#![no_std]`). The standard library is `libsarga`, located at `libsarga/`. There is no C libc.

## Entry Point

Every program exports `main()` via the `sarga_main!` macro:

```rust
#[no_std]
#[no_main]
extern crate alloc;
use libsarga::{println, sarga_main};

fn user_main() -> i32 {
    println!("hello");
    0
}
sarga_main!(user_main);
```

`libsarga` provides the `#[panic_handler]` and re-exports `alloc`.

## Standard I/O (`io`, `stdio`)

- `println!` / `print!` / `fprintf!` macros
- `libsarga::io::*` — raw syscall-backed I/O

## Filesystem (`fs`)

```rust
pub fn open(path: &str, flags: u64) -> Result<i64, i64>;
pub fn read(fd: i64, buf: &mut [u8]) -> Result<usize, i64>;
pub fn write(fd: i64, buf: &[u8]) -> Result<usize, i64>;
pub fn close(fd: i64) -> i64;
pub fn stat(path: &str) -> Result<Stat, i64>;
pub fn statfs(path: &str) -> Result<StatFs, i64>;
pub fn read_to_string(path: &str) -> Result<String, i64>;
pub fn write_file(path: &str, content: &str) -> Result<(), i64>;
pub fn mount(source: &str, target: &str, fstype: &str, flags: u64) -> Result<(), i64>;
pub fn umount(target: &str) -> Result<(), i64>;
pub fn mkfs(fstype: &str, device: u64) -> Result<(), i64>;
```

## Memory (`mem`)

`malloc`/`calloc`/`realloc`/`free` wrappers over the kernel heap + `mmap`/`munmap` syscalls.

## Processes & Threads (`process`, `thread`, `pthread`, `signal`, `sync`)

- `process::exit`, `process::spawn`, `fork`/`exec`/`wait` wrappers
- `thread::*`, `pthread::*` threading primitives
- `signal::*` — signal handlers (see `docs/security/syscall_security.md`)
- `sync::*` — mutex/condvar

## Networking (`net`) & IPC (`ipc`)

- `net::*` — socket API (AF_INET/AF_INET6, TCP/UDP; see `docs/socket-api.md`)
- `ipc::*` — userland message passing / service RPC

## Graphics & GUI (`gui`, `glass`, widget modules)

- `gui::*` — window/event syscall wrappers (see `docs/api/gui_syscalls.md`)
- `glass::*` — hardware compositor API (Vahi-Glass)
- Widget toolkit: `button`, `checkbox`, `combobox`, `dialog`, `label`, `layout`, `menubar`, `progress_bar`, `scrollbar`, `slider`, `tab_widget`, `textbox`, `theme`, `widget`

## Misc (`toml`, `serialize`, `semver`, `uuid_util`, `random`, `hash`, `regex_util`, `datetime`, `vahiai`)

TOML parser, serialization helpers, semver, UUIDs, RNG, hashing, regex helpers, chrono-backed dates, and the Vahiai LLM query helper.

## Errors

Syscall wrappers return `Result<T, i64>` with a negative errno; `libsarga::errno` has `set_errno`/`get_errno` for C-style code paths.

## Full Module List

`ai`, `args`, `config`, `datetime`, `errno`, `error`, `fs`, `gpu`, `gui`, `hash`, `init`, `init_services`, `io`, `ipc`, `libskyos`, `mem`, `net`, `posix`, `process`, `pthread`, `random`, `regex_util`, `semver`, `serialize`, `signal`, `start`, `stdio`, `sync`, `syscall`, `thread`, `time`, `toml`, `uuid_util`, `vahiai`, `version`, `glass`, plus the widget toolkit.
