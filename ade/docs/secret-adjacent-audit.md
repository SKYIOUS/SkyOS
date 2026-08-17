# Secret-adjacent programs: echo forward-compat + plaintext-storage audit

**Status:** AUDIT ONLY — read-only source inspection, no code changed.
**Date:** Aug 12, 2026. **Scope:** `sash` (history save + input paths),
`aicli` (tokens), `spkg` / `skystore` (auth), compared against the
getty/login termios-echo contract pinned in `tests/test_login_echo.py`.
Every claim carries a file:line citation against the live tree.

Two axes, mirroring the getty work:

1. **Forward-compat termios echo risk** — does the program read the console
   tty in a way that silently depends on the kernel's default `c_lflag`
   (the `0xB` `ECHO` default spec'd in `kernel-tcsets-echo.md`), or that
   would double-echo when the kernel lands ECHO-by-default?
2. **Plaintext secret storage on disk** — are typed secrets or auth
   material persisted without masking, filtering, or restrictive modes?

---

## 1. sash — command history is written in PLAINTEXT with no mode

**The finding.** Every interactive command is stored in memory
(`sash/src/readline.rs:9-13`, `History { entries: Vec<String>, ... }`; the
main loop locks the history and feeds every input through the editor at
`sash/src/main.rs:475-479`) and written verbatim to `$HOME/.sash_history`
on the `exit` builtin (`sash/src/builtins.rs:19` →
`sash/src/main.rs:343-347` `save_history_on_exit` → `History::save`).

`History::save()` (`sash/src/readline.rs:122-139`):

```rust
let path = format!("{}/.sash_history", home);          // :124
let fd = unsafe { libsarga::syscall::syscall2(2, c_str.as_ptr() as u64, 0x241u64) };
                                                       // :129  0x241 = O_WRONLY(1)|O_CREAT(0x40)|O_TRUNC(0x200)
                                                       //       (libsarga/src/posix.rs:13-16)
for entry in &self.entries {
    let mut line = entry.clone();                      // :133-137 verbatim, no filtering
    line.push('\n');
    let _ = libsarga::io::write(fd, line.as_bytes());
}
```

- **Plaintext, no secret filtering** (`:133-137`): a command such as
  `echo secret | app` or `app --password x` lands in the file byte-for-byte.
- **No restrictive mode — a kernel gate.** The open call is the raw
  2-arg `syscall2` with no `mode`; `libsarga::io::open(path, flags)` is
  likewise 2-arg (`libsarga/src/io.rs:84`). The only mode-capable userspace
  wrapper is `openat` (`io.rs:280`, 4-arg) — **and no current code path
  uses it** for history, so `~/.sash_history` is created with the kernel's
  O_CREAT default, which the audit could not establish as restrictive.
  (A future program *could* pass `0o600` through `openat` — the gap is
  that the 2-arg `open`/`syscall2` path used here can't.) The rewrite
  should either teach `open`/the syscall a mode or have sash switch the
  history write to `openat(path, O_WRONLY|O_CREAT|O_TRUNC, 0o600)`.
- **Save only on `exit`** (`builtins.rs:19`): EOF-terminated shells skip the
  save — inconsistent, but the leak path (typing `exit`) is the common one.
- `History::load()` (`readline.rs:93-118`) round-trips the same plaintext
  file back into memory at startup (`main.rs:447`).

**Forward-compat echo axis:** the editor reads fd 0 raw and self-draws; the
kernel-ECHO double-echo interaction is already documented as mandatory in
`kernel-tcsets-icanon.md` §4 (the editor must clear `ICANON|ECHO` via
TCSETS). No new finding here beyond that spec.

---

## 2. aicli — NO tokens exist; it is an fd-0 console reader

**Premise corrected.** `aicli/src/main.rs` is a 28-line REPL:

- `:13-14` — `let n = io::read(0, &mut input).unwrap_or(0);` reads the
  console tty directly, no termios/TCSETS handling, no self-echo;
  `if n == 0 { break; }` (`:14`) treats an empty read as EOF.
- `:21` — input goes to `libsarga::vahiai::handle_intent(s)`, a local
  keyword-matching assistant (`libsarga/src/vahiai.rs:77-82` — matches
  "proc"/"process", "mem"/"memory", …). No network, no HTTP, no API.
- Grep across `aicli/src/` for `token|key|secret|auth|api|config|.json|
  .toml|env` returns **zero hits** — there is nothing to leak.

**Forward-compat echo axis (real, if low-severity):** like login's username
read before `ensure_echo`, aicli depends on the kernel default for
visibility — with ECHO on (`0xB`) typing echoes normally; with a
non-echoing default it is invisible. It is also hit by the ICANON spec's
D0 finding (`kernel-tcsets-icanon.md` §2): the current non-blocking tty
read returns `0` on an empty queue, and `if n == 0 { break; }` (`:14`)
would make aicli exit at its first prompt. aicli belongs in that spec's
reader table (console fd-0 reader needing D0; add `ensure_echo`-style
TCSETS if interactive visibility matters).

---

## 3. spkg — no secrets; two hygiene notes

- Grep across `spkg/src/` for `token|password|secret|auth|credential`:
  **zero hits.** All writes are package metadata:
  - `spkg/src/db.rs:6,97-98` — `/var/spkg/installed.toml`,
    `io::open(DB_PATH, 0x42)` = `O_RDWR|O_CREAT`, **no mode**.
  - `spkg/src/repo.rs:72-80` — `/var/spkg/cache/<name>.toml` index caches,
    `io::open(..., 0x42)`, no mode; `spkg/src/install.rs:37` — same pattern.
  - Same no-mode kernel gate as sash: metadata files get the kernel's
    O_CREAT default.
- **URL-credential note:** repo URLs from `/etc/spkg/repos.conf`
  (`spkg/src/main.rs:27`) are parsed verbatim (`spkg/src/repo.rs:60-66`) and
  used to build `index_url` (`repo.rs:69`). The fetch diagnostic prints the
  **full URL** to serial (`main.rs:34` — `"fetching index from {} ({})..."`).
  An operator who embeds `https://user:pass@host/` in the conf would have
  those credentials echoed to the serial log. Plaintext conf storage is the
  operator's choice (standard for apt-style repo configs), but the print is
  avoidable.

---

## 4. skystore — clean

- Input is GUI-only: `win.get_key()` (`skystore/src/main.rs:371, 430`), the
  same `SYS_GUI_GET_KEY` pipeline as login-manager — no tty, no termios.
- Reads `/packages/` metadata via `list_dir`/`read_to_string`
  (`:271-279`), parses `key = value` pairs from package info (`:243-245`).
- No auth, no tokens, no secret material, no persistent writes of anything
  sensitive.

---

## 5. Summary table

| Program | Input path | termios/echo refs | Kernel-ECHO dependency | Plaintext on disk |
|---|---|---|---|---|
| sash editor | fd 0 raw (`readline.rs:811`, per `kernel-tcsets-icanon.md` §4) | none | self-draws → double-echo when kernel ECHO lands; must clear `ICANON\|ECHO` | **YES** — `$HOME/.sash_history` verbatim, no mode (`readline.rs:122-139`) |
| sash `read_raw_line` | fd 0 (`main.rs:565-570`) | none | inherits default (no secret input) | via `exit`-saved history only |
| aicli | fd 0 (`main.rs:20-21`) | none | inherits default; invisible typing if default non-echo; **breaks on D0** (`n==0 → break`) | **no secrets exist** (premise corrected) |
| spkg | none (no tty input) | none | n/a | metadata only (installed.toml, index caches) — no mode on O_CREAT; repo-URL creds could reach serial via `main.rs:34` |
| skystore | `win.get_key()` (`:371,430`) | none | n/a (GUI, like login-manager) | none |

---

## 6. Forward-compat actions (for when the kernel lands ECHO/ICANON/TCSETS)

1. **sash history (highest priority):** write `$HOME/.sash_history` via
   `openat(path, O_WRONLY|O_CREAT|O_TRUNC, 0o600)` (`libsarga/src/io.rs:280`
   already carries mode) or add a mode to the kernel open; filter lines that
   contain secrets (`password`, `--token`, …) before saving; save on normal
   shell exit too (today only the `exit` builtin saves).
2. **sash editor:** clear `ICANON|ECHO` (raw) at editor start — already the
   documented mandatory companion change in `kernel-tcsets-icanon.md` §4.
3. **aicli:** add `ensure_echo`-style TCSETS at startup if it stays an
   fd-0 console reader; add it to the ICANON spec's D0 reader list.
4. **spkg:** stop printing `repo.url` on fetch (`main.rs:34`) or redact
   embedded credentials; use `openat(..., 0o600)` for metadata if
   permission hygiene is desired.
5. **skystore:** nothing — clean as-is.

## 7. Not checked here (out of scope)

- The kernel-side clipboard (`SYS_CLIPBOARD`) contents — covered by the
  F1 clipboard rewire audit, not a disk-storage issue.
- Passphrase material in `/etc/shadow` — read-only consumers (`verify_password`),
  hash-at-rest, no plaintext writes.
- Network auth for any future aicli backend — none exists today.
