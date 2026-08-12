# Kernel DRMCTL shape fix (set_mode / map_dumb)

**Status:** SPEC ONLY — a draft for the kernel rewrite. The kernel is under major
change; nothing here applies yet. The only userspace change so far is doc marking:
`libsarga/src/gpu.rs` `set_mode`/`map_dumb` are annotated UNUSABLE until this lands.

**Queue:** this is **K5** in [session-lifecycle.md](session-lifecycle.md) §6
(kernel change queue). Line numbers below drift; function names, syscall numbers,
and struct layouts are the stable anchors.

## 1. The broken shape (evidence)

`SYS_DRMCTL` forwards only three syscall args:

```rust
// syscalls/mod.rs:756
numbers::SYS_DRMCTL => sys_drmctl(arg1, arg2, arg3 as *mut u8),
// syscalls/mod.rs:6026
fn sys_drmctl(_fd: u64, request: u64, arg: *mut u8) -> u64 {
```

- **SET_MODE always fails with EINVAL.** libsarga calls
  `syscall5(SYS_DRMCTL, 0, DRM_SET_MODE, w, h, bpp)` (`gpu.rs:88`) → the kernel sees
  `_fd=0, request=0x0105, arg=w`. The SET_MODE arm reads
  `new_w = _fd as usize` (=0) and `new_h = request as usize` (=0x0105 = 261)
  (`syscalls/mod.rs:6080-6081`), both outside the `640..=3840` / `480..=2160` ranges
  (`:6082`) → **permanent EINVAL**. The arm's own comment ("passed as direct args from
  userspace") describes a contract no caller meets. The only consumer,
  `sargasettings/src/main.rs:162` (`let _ = libsarga::gpu::set_mode(w, h, 32)`),
  silently swallows the error — resolution selection in the settings app can never
  change the mode.
- **MAP_DUMB returns the wrong pointer for any id.** libsarga calls
  `syscall3(SYS_DRMCTL, id, DRM_MAP_DUMB, 0)` (`gpu.rs:97`); the kernel MAP_DUMB arm
  ignores the id entirely and returns the main framebuffer vaddr
  (`syscalls/mod.rs:6095-6097`). Dumb buffers are `Box::leak`ed (`:6058`) with no
  mapping table, so "map" is a no-op lie. DESTROY_DUMB is a no-op (`:6062`) and
  libsarga's `destroy_dumb()` passes no id (`syscall2(SYS_DRMCTL, 0, DRM_DESTROY_DUMB)`,
  `gpu.rs:78`).

Working arms for contrast (struct/color/path in `arg`, copied via `user_access`):
GET_DISPLAY_INFO, CREATE_DUMB, FLIP, PAGE_FLIP, GEM_CREATE, GEM_MMAP, ACCENT_COLOR,
WALLPAPER. Only SET_MODE and MAP_DUMB (and DESTROY_DUMB's no-id signature) are
shape-broken.

## 2. Fix 1 — SET_MODE reads a struct pointer in `arg` (CREATE_DUMB pattern)

**Kernel** — replace the SET_MODE arm body (`syscalls/mod.rs:6078-6092`):

```rust
DRM_IOCTL_SET_MODE => {
    #[repr(C)]
    struct ModeInfo { width: u32, height: u32, bpp: u32 }
    let mut mi = ModeInfo { width: 0, height: 0, bpp: 0 };
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(
            &mut mi as *mut ModeInfo as *mut u8,
            core::mem::size_of::<ModeInfo>(),
        )
    };
    // width/height arrive in a userspace struct via `arg`, matching the
    // GET_DISPLAY_INFO / CREATE_DUMB pattern (replaces the _fd/request misread).
    if unsafe { user_access::copy_from_user(bytes, arg) }.is_err() {
        return errno::Errno::EFAULT as u64;
    }
    if !(640..=3840).contains(&(mi.width as usize))
        || !(480..=2160).contains(&(mi.height as usize))
    {
        return errno::Errno::EINVAL as u64;
    }
    crate::drivers::gpu::set_mode(mi.width, mi.height);
    crate::drivers::graphics::WIDTH.store(mi.width as usize, core::sync::atomic::Ordering::SeqCst);
    crate::drivers::graphics::HEIGHT.store(mi.height as usize, core::sync::atomic::Ordering::SeqCst);
    crate::drivers::graphics::STRIDE.store(mi.width as usize, core::sync::atomic::Ordering::SeqCst);
    crate::gui::COMPOSITOR.lock().set_resolution(mi.width as usize, mi.height as usize);
    crate::println!("DRM: set_mode {}x{}", mi.width, mi.height);
    0
}
```

`copy_from_user(dst: &mut [u8], src_ptr: *const u8) -> Result<(), ()>` already exists
(`syscalls/user_access.rs:153`) and is the read-direction twin of the `copy_to_user`
GET_DISPLAY_INFO/CREATE_DUMB use. `bpp` is carried for ABI stability but unused by
`drivers::gpu::set_mode(w, h)`; the arm may ignore it or validate `== 32`.

**libsarga** — `set_mode` passes the struct by pointer (public signature unchanged):

```rust
#[repr(C)]
pub struct ModeInfo {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
}

pub fn set_mode(w: u32, h: u32, bpp: u32) -> Result<(), i64> {
    let info = ModeInfo { width: w, height: h, bpp };
    let r = unsafe { syscall3(SYS_DRMCTL, 0, DRM_SET_MODE, &info as *const ModeInfo as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}
```

## 3. Fix 2 — MAP_DUMB: a real id → vaddr registry

Add a kernel-side registry (same pattern as `COMPOSITOR`:
`lazy_static` + `crate::sync::IrqSafeMutex`, `gui/mod.rs:20,32-33`):

```rust
// syscalls/mod.rs (module level)
lazy_static::lazy_static! {
    /// (dumb id, leaked framebuffer vaddr) — CREATE_DUMB registers,
    /// DESTROY_DUMB removes, MAP_DUMB looks up.
    static ref DUMB_BUFFERS: Mutex<alloc::vec::Vec<(u64, *mut u32)>> =
        Mutex::new(alloc::vec::Vec::new());
}
static NEXT_DUMB_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);
```

**CREATE_DUMB** — register a unique id (replaces the hardcoded `id: 1`):

```rust
DRM_IOCTL_CREATE_DUMB => {
    let w = crate::drivers::gpu::width();
    let h = crate::drivers::gpu::height();
    let fb: &'static mut [u32] = Box::leak(alloc::vec![0u32; (w * h) as usize].into_boxed_slice());
    let id = NEXT_DUMB_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    DUMB_BUFFERS.lock().push((id, fb.as_mut_ptr()));
    let paddr = crate::memory::virt_to_phys(VirtAddr::from_ptr(fb.as_ptr())).unwrap().as_u64();
    #[repr(C)]
    struct DumbInfo { id: u64, size: u64, addr: u64 }
    let di = DumbInfo { id, size: (w * h * 4) as u64, addr: paddr };
    // ... copy_to_user(di) exactly as today ...
}
```

**MAP_DUMB** — honor the id (`_fd` carries it; libsarga already passes it):

```rust
DRM_IOCTL_MAP_DUMB => {
    let id = _fd;
    let bufs = DUMB_BUFFERS.lock();
    match bufs.iter().find(|(i, _)| *i == id) {
        Some((_, vaddr)) => *vaddr as u64,
        None => errno::Errno::ENOENT as u64,
    }
}
```

**DESTROY_DUMB** — remove from the registry; libsarga's `destroy_dumb` gains an id:

```rust
DRM_IOCTL_DESTROY_DUMB => {
    let id = _fd;
    let mut bufs = DUMB_BUFFERS.lock();
    bufs.retain(|(i, _)| *i != id);
    0
}
```

```rust
// libsarga gpu.rs — signature change: the id is now required to free the right buffer.
pub fn destroy_dumb(id: u64) -> Result<(), i64> {
    let r = unsafe { syscall2(SYS_DRMCTL, id, DRM_DESTROY_DUMB) };
    if r < 0 { Err(-r) } else { Ok(()) }
}
```

**ABI note for the rewrite:** CREATE_DUMB's `DumbInfo.addr` is the buffer's PHYSICAL
address (`virt_to_phys`, `syscalls/mod.rs:6060`) while MAP_DUMB returns a VIRTUAL
address — two different address spaces in one ABI. Recommend MAP_DUMB as the canonical
map path (return the vaddr, which is what userspace writes) and either drop `addr` from
`DumbInfo` or document it as informational-only.

**Allocation note:** the `Box::leak` stays (the registry makes lookup/destroy correct);
real deallocation on DESTROY_DUMB requires a tracked allocation instead of the leak —
out of scope for this shape fix, but the registry makes it a drop-in.

## 4. Verification (how the rewrite proves it)

- **Kernel selftest** (framework: `kernel/src/selftest.rs`, TAP, registered in
  `tests/mod.rs` like `memory_tests.rs`):
  - `gui::drmctl_set_mode_ok` — build `ModeInfo { width: 1920, height: 1080, bpp: 32 }`,
    call `sys_drmctl(0, 0x0105, &mi)`, assert `0` returned and
    `drivers::graphics::WIDTH/HEIGHT` updated.
  - `gui::drmctl_map_dumb_roundtrip` — `CREATE_DUMB` → non-zero id; `MAP_DUMB(id)`
    returns a non-null pointer that is **not** the main framebuffer;
    `DESTROY_DUMB(id)`; `MAP_DUMB(id)` returns `-ENOENT`.
  - Timing caveat: `set_mode` touches `COMPOSITOR` and the GPU driver — run these
    post-`gui::init` or guard on driver presence (same constraint as the K3 spec).
- **QEMU / sargasettings:** selecting a resolution in the settings app must print
  `DRM: set_mode WxH` (the arm already prints it) instead of failing EINVAL; a GUI
  harness can grep that line.
- **Un-gate:** remove the UNUSABLE doc comments from `libsarga/src/gpu.rs`
  (`set_mode`, `map_dumb`) once the above passes.

## 5. Landing condition (queue K5)

| Proof | Marker |
|---|---|
| Kernel selftest | `ok N - gui::drmctl_set_mode_ok` and `ok N - gui::drmctl_map_dumb_roundtrip` in self_test serial |
| Settings app | `DRM: set_mode` in the QEMU serial log after a resolution change |
| libsarga | UNUSABLE comments removed from `gpu.rs` `set_mode`/`map_dumb` |
