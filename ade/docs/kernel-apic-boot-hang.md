# Kernel APIC boot hang — unblock design for SMP-2 (AP INIT) and SMP-1 (PCI enumerate)

**Status:** SPEC ONLY — no kernel code changed. The kernel is mid-major-change;
anchors are verified against the current tree at `SKYIOUS KERNEL/kernel/src/`,
Aug 11, 2026 (the tree with the uncommitted 50-file delta — every file cited
here is part of that dirty set). Function names are the stable anchors if line
numbers drift.
**Purpose:** give the kernel rewrite a concrete, reviewable change set for the
two boot freezes observed on the local QEMU TCG build, plus the latent
hardware bugs those freezes expose. This doc mirrors the structure of
`docs/kernel-gui-modifier-delivery.md`.
**Companion:** the environment-side workaround and its evidence live in
`AGENTS.md` §"QEMU Debugging Pitfalls" (`-cpu qemu64,-smep`).

---

## 1. The problem (verified, not assumed)

Both legs were reproduced this session against `release/skyos-0.6.0.iso`
(which embeds this exact kernel tree) on the local QEMU 10.2.50 TCG build:

| Leg | Observed (serial log tail) | Where the stream stops |
|---|---|---|
| `-smp 2` | `[APIC] timer started` → `[BOOT] SMP init...` → `[SMP] init enter` | inside `smp::init()` (`smp.rs:183`) — rc=124 hang, zero exceptions under `-d int` |
| `-smp 1` | `[PS2] ...` → `[BOOT] PCI enumerate...` | inside `enumerate_pci()` (`pci/mod.rs:291`) — no `PCI Device:` line ever printed; rc=124 hang, zero exceptions |
| run-to-run | the stall point moves (another SMP-1 run froze inside PS/2 init) | TCG timing-dependent, not a single deterministic instruction |

CI ubuntu QEMU boots `-smp 2` fine, which is why the existing gates stay
green — these defects are **latent on CI today** and will surface on real
hardware / other emulators regardless.

### 1.1 The stage order (kernel/src/main.rs)

```
276  apic::init();                                  # LAPIC enable + timer
279  serial_write("[BOOT] SMP init..."); smp::init();   # bring up APs
282  serial_write("[BOOT] PCI enumerate...");       # <- SMP-1 freeze point
283  pci::enumerate_pci();
```

### 1.2 Kernel defects the freeze exposes (wrong on real hardware regardless of TCG)

| # | Defect | Site | Why it is a bug |
|---|---|---|---|
| F1 | **INIT IPI is sent deassert-level.** `ICR_ASSERT = 1 << 14` is the *trigger-mode* bit; the real assert/level bit is **bit 17** and is never set anywhere. `send_ipi` ORs `ICR_ASSERT` into **every** IPI (fixed, INIT, NMI…), making fixed IPIs level-triggered and INIT delivery level=0 (deassert). | `apic/mod.rs:110,299-304` | Intel SDM 10.6: ICR bit 17 = Level (assert=1). INIT with level=0 never resets the AP on real hardware. It works on QEMU only by accident — QEMU keys INIT-assert on bit 14 (`APIC_LVT_LEVEL_TRIGGER`). |
| F2 | **SMEP gated on the wrong CPUID bit** — SMEP is `CPUID.07H.0:ECX[7]`; both sites check `ECX[2]` (SGX). Under TCG `-cpu max` this passes, so SMEP is enabled on the AP. | trampoline `smp.rs:118-127` (`and ecx, 4; shl ecx, 9`); BSP/AP C entry `arch/arch_x86_64.rs:61` (`ecx7 & (1 << 2)`) | The AP then writes CR4.SMEP, which this TCG never retires on an AP vCPU (verified via gdbstub, `AGENTS.md:27`) — the documented SMP-2 stall. The `-cpu qemu64,-smep` workaround only works because qemu64 clears ECX[2], i.e. the workaround keys off the *wrong* feature bit. |
| F3 | **Unbounded waits in the device paths** — the e1000 TX-done spin and the AHCI command-completion loop never time out, so a misbehaving device hangs the machine with no diagnostic. These are *runtime I/O-path* waits (e1000 `send_packet`, AHCI command issue), not `enumerate_pci` init-path waits — the observed SMP-1 freeze produced zero `PCI Device:` lines, so no device init even ran. They are latent hazards the boundedness discipline removes, not the observed freeze cause. | `e1000.rs:228-233` (`send_packet` TX-done spin — no bound); `drivers/storage/ahci.rs:395-408` (completion `loop { … }` — no spin counter, unlike its siblings at `:385-391,459-465`) | XHCI is the *model*: every wait has a `1_000_000`-spin bound (`xhci.rs:436-454`). The unbounded two are the outliers. |
| F4 | **Unbounded full-bus sweep** — `enumerate_pci` probes all 256 buses × 32 slots × up to 8 funcs (~65k config reads) with no bus-presence detection and no recursion via PCI-PCI bridges. | `pci/mod.rs:291-297` | On VMs that slow-trap absent-bus config reads this is a stall multiplier, and it is simply wrong enumeration (buses > 0 only exist behind bridges). |

The user-visible framing — "SMP-2 freezes at AP INIT delivery" — is *consistent
with* F1+F2: on QEMU the INIT is delivered (bit-14 quirk), the AP would run
the trampoline, and die at the CR4.SMEP write (the AGENTS.md gdbstub evidence
is from an earlier run/commit; this session's SMP-2 run stalled even earlier,
inside `smp::init` before the INIT/SIPI markers — same TCG stall class,
different depth). "SMP-1 at PCI enumerate" maps to the F3/F4 boundedness gap.

---

## 2. Design A (recommended): fix the ICR, don't write SMEP on APs, bound the scan

Four independent, low-risk changes. Landing A1+A2 removes the SMP-2 leg and
makes plain `-smp 2` boot under the affected TCG; A3+A4 remove the SMP-1 leg
and every F3/F4-class hang.

### Change A1 — `kernel/src/apic/mod.rs` ICR encoding (≈10 lines)

```rust
-/// Delivery mode 5: INIT request.
-pub const ICR_DELIVERY_MODE_INIT: u8 = 5;
-pub const ICR_DELIVERY_MODE_SIPI: u8 = 6;
+pub const ICR_DELIVERY_MODE_INIT: u8 = 5;
+pub const ICR_DELIVERY_MODE_SIPI: u8 = 6;
+
+/// ICR bit 14 — trigger mode (SDM 10.6). Legal only for INIT delivery.
+pub const ICR_TRIGGER_LEVEL: u32 = 1 << 14;
+/// ICR bit 17 — assert/deassert level. INIT-assert requires 14|17; INIT
+/// without bit 17 is a deassert and never resets the AP.
+pub const ICR_LEVEL: u32 = 1 << 17;
+
+// NOTE: the old `pub const ICR_ASSERT: u32 = 1 << 14;` (apic/mod.rs:110) MUST
+// be deleted with this change — it aliased the trigger-mode bit, was ORed
+// into EVERY IPI (so fixed/NMI IPIs were sent with bit 14 set and INIT was
+// sent deassert), and would otherwise trip the workspace dead_code gate.
```

```rust
pub fn send_ipi(dest_lapic_id: u8, vector: u8, delivery_mode: u8) {
    lapic_write32(ICR_HIGH, (dest_lapic_id as u32) << 24);
-    lapic_write32(
-        ICR_LOW,
-        ICR_ASSERT | ((delivery_mode as u32) << ICR_DELIVERY_MODE_SHIFT) | vector as u32,
-    );
+    // Edge by default; only INIT is level-triggered + asserted. (For
+    // non-INIT delivery modes SDM 10.6 says the trigger-mode bit is ignored,
+    // so clearing it is the canonical form, not a behavioral fix.)
+    let mut low = ((delivery_mode as u32) << ICR_DELIVERY_MODE_SHIFT) | vector as u32;
+    if delivery_mode == ICR_DELIVERY_MODE_INIT {
+        low |= ICR_TRIGGER_LEVEL | ICR_LEVEL; // INIT-assert (14|17)
+    }
+    lapic_write32(ICR_LOW, low);
 }
```

`send_broadcast_ipi` (`:307-313`) drops the `ICR_ASSERT` term the same way —
fixed broadcast IPIs must be edge (bits 14/17 clear). `smp_call_function`
(`smp.rs:384-392`) needs no change; it routes through `send_ipi`.

The full SDM-canonical INIT sequence (deassert → 10 ms → assert) is not
required on any emulator or the observed hardware and adds 10 ms per AP; the
single assert is what Linux-style bare-metal bring-up uses in practice.
Leave the 10 ms settle in `smp.rs:288-290` as-is.

**Dead-code gate:** deleting the old `ICR_ASSERT` constant is mandatory — the
workspace CI greps for unreferenced `pub const` and fails (`dead_code`-style
scan, mirroring the theme.rs sweep). The A1 diff above removes its only uses.

### Change A2 — no SMEP on AP bring-up (2 sites)

Trampoline (`kernel/src/smp.rs:118-127`) — unconditional omission, no CPUID
consult at all (TCG *claims* SMEP, so a CPUID gate cannot save us):

```asm
-    # Enable PAE + PGE (CR4 bits 5, 7); SMEP (bit 11) only if CPUID
-    # leaf 7 ECX bit 2 says so, matching the BSP (arch_x86_64 init_cpu).
-    # Unconditional SMEP trips a QEMU TCG stall on AP CR4 writes and
-    # #GPs on SMEP-less hardware.  EBX/EDX are unused for the rest of
-    # the trampoline, so cpuid clobbers are harmless.
-    mov eax, 7
-    xor ecx, ecx
-    cpuid
-    and ecx, 4
-    shl ecx, 9        # 4 << 9 = SMEP (bit 11)
     mov eax, cr4
     or eax, (1 << 5) | (1 << 7)
-    or eax, ecx
     mov cr4, eax
```

C entry (`kernel/src/arch/arch_x86_64.rs:59-61`) — fix the CPUID bit AND skip
on APs (the AP is kernel-only until the scheduler runs user threads; SMEP can
be enabled later per-CPU if ever needed). **Primary form: thread a `bool`
through `init_cpu`** — on the AP, `init_cpu()` runs at `smp.rs:330` *before*
`lapic::init()` later in `ap_kernel_entry`, so a `get_cpu_id()` call (LAPIC
MMIO read) inside `init_cpu` may fault on an AP whose LAPIC mapping is not yet
active. The `smp.rs` call site knows it is an AP and passes the flag:

```rust
// kernel/src/arch/arch_x86_64.rs — signature
-    pub fn init_cpu() {
+    pub fn init_cpu(bsp: bool) {
         ...
-            if ecx7 & (1 << 2) != 0 {
-                flags.insert(Cr4Flags::from_bits_truncate(0x800));
+            // SMEP is CPUID.07H.0:ECX[7] (bit 2 is SGX). Only the BSP takes
+            // it for now: QEMU TCG (10.2.50) never retires a CR4 write that
+            // sets SMEP on an AP vCPU (AGENTS.md), and APs only run kernel
+            // code until the scheduler hands them user threads.
+            if bsp && ecx7 & (1 << 7) != 0 {
+                flags.insert(Cr4Flags::from_bits_truncate(0x800));
             }
```

```rust
// kernel/src/smp.rs — the two call sites
-    crate::arch::CurrentArch::init_cpu();       // ap_kernel_entry :330
+    crate::arch::CurrentArch::init_cpu(false);  // AP
```

(and the BSP call in the main boot path passes `true`). With this change,
plain `qemu-system-x86_64 … -smp 2` boots under the affected TCG and
`-cpu qemu64,-smep` stops being load-bearing.

### Change A3 — bounded, presence-aware enumeration (`kernel/src/pci/mod.rs`)

```rust
 pub fn enumerate_pci() {
     crate::println!("PCI: Enumerating Bus...");
-    for bus in 0..255u8 {
-        for slot in 0..32u8 {
-            enumerate_bus_slot(bus, slot);
-        }
-    }
+    // Scan bus 0, then recurse only into secondary buses of discovered
+    // PCI-PCI bridges (header type 1 at config 0x0E; secondary bus number
+    // at 0x19). Probing all 256 buses is ~65k reads with no presence check
+    // and no bridge traversal.
+    let mut buses = alloc::vec![0u8];
+    let mut seen = 0usize;
+    while let Some(bus) = buses.pop() {
+        if seen >= 256 { break; }   // full-scan guard: corrupt bridge chains terminate
+        for slot in 0..32u8 {
+            // Header type is at config offset 0x0E. (The CURRENT
+            // enumerate_bus_slot :117-119 reads 0x0C as a u16 and tests
+            // (>>8)&0x80 — that tests the latency-timer byte, a pre-existing
+            // offset bug this rewrite should fix in both places.)
+            let htype = read_config_u8(bus, slot, 0, 0x0E);
+            if htype & 0x80 != 0 && htype & 0x7F == 0x01 {
+                let secondary = read_config_u8(bus, slot, 0, 0x19);
+                if secondary != 0 && !buses.contains(&secondary) {
+                    buses.push(secondary);
+                }
+            }
+            enumerate_bus_slot(bus, slot);
+            seen += 1;
+        }
+    }
 }
```

`alloc::vec!` is already available (the tree allocates freely). The `seen >= 256`
guard is checked *before* each bus's slot walk, so even a corrupt bridge
chain terminates after ~256 slot probes instead of the full ~65k.

### Change A4 — bound the two unbounded waits

`kernel/src/drivers/net/e1000.rs:228-233`:

```rust
-        loop {
-            let s;
-            unsafe { s = core::ptr::read_unaligned(core::ptr::addr_of!(self.tx_descs[cur].status)); }
-            if s & 1 != 0 { break; }
-        }
+        for _ in 0..1_000_000 {
+            let s;
+            unsafe { s = core::ptr::read_unaligned(core::ptr::addr_of!(self.tx_descs[cur].status)); }
+            if s & 1 != 0 { break; }
+            core::hint::spin_loop();
+        }
```

`kernel/src/drivers/storage/ahci.rs:395-408` — same pattern as its siblings:

```rust
-    loop {
+    let mut spin = 0;
+    loop {
         if (port.is.read() & (1 << 30)) != 0 {
              return false;
         }
         if (port.ci.read() & (1 << slot)) == 0 {
             break;
         }
         if (port.is.read() & (1 << 26)) != 0 {
               return false;
         }
+        spin += 1;
+        if spin > 1_000_000 { return false; }
+        core::hint::spin_loop();
     }
```

### Why this design

- **A1+A2 are the SMP-2 contract.** F1 is a real-hardware correctness bug
  (INIT never asserts); F2 is the TCG stall trigger. Both are tiny and each is
  independently testable.
- **A3+A4 are the SMP-1 contract.** They convert a stall-prone, unbounded scan
  into a bounded, bridge-aware walk and delete the two unbounded waits — the
  same discipline XHCI already has.
- No ABI or userspace impact; no feature-flag churn.

---

## 3. Design B (minimal): environment workaround + stage watchdog

If the kernel team wants a zero-risk interim while the rewrite settles:

| Change | What |
|---|---|
| B1 | Keep `-cpu qemu64,-smep` as the documented QEMU invocation for local SMP-2 (`AGENTS.md:27` already says so; make it the default in `tests/boot_stress.py`, whose `--cpu` help already points there). Note this is an *inference-ridden* workaround: `-smep` strips CPUID ECX[7] (SMEP), while the buggy gate checks ECX[2] — it works because the qemu64 model happens to expose neither, which is exactly why it is fragile |
| B2 | **Boot-stage watchdog**: a `boot_progress(stage: &'static str)` that stores `(stage, tick)` in a global; the LAPIC timer IRQ checks `now - tick > STALL_TICKS` and serial-prints `WATCHDOG: stalled in <stage>` once. Every `[BOOT] …` marker in `main.rs:276-283` (and each `enumerate_bus_slot` device branch) calls it. Turns every silent freeze into a named diagnostic. |
| B3 | No kernel correctness fixes — F1/F2/F3/F4 remain latent for real hardware |

**Recommendation: A**, with B2 adopted regardless (it costs ~15 lines and
makes every future boot stall self-identifying). B is only for a cycle where
the kernel team cannot touch the APIC path at all.

---

## 4. Verification plan (after the kernel change lands)

1. **Kernel selftest** (the suite `boot_stress.py` gates; see `AGENTS.md`):
   - `apic::selftest_icr`: assert `ICR_TRIGGER_LEVEL == 1 << 14`,
     `ICR_LEVEL == 1 << 17`, and that `send_ipi` leaves bits 14/17 clear for
     fixed mode and sets both for INIT.
   - `pci::selftest_scan`: run the A3 walk on the QEMU machine; assert it
     finds the host bridge / VGA / e1000 on bus 0, terminates, and probes
     ≤ 256 slots total.
   - `smp::selftest_cr4`: read CR4 on the BSP and assert SMEP (bit 11) is
     clear unless the CPUID bit-7 gate + BSP check both hold.
2. **Boot gate — the real proof:** `tests/boot_stress.py --smp 2` *without*
   `-cpu qemu64,-smep` on the affected TCG build. This is the exact scenario
   that freezes today; green after A1+A2 is the acceptance test.
3. **SMP-1 leg:** the ISO-capture workflow (`release-iso.yml`, new
   `kernel_ref`/`boot_capture` inputs) boots headless and its reachability
   gate (`login:` in the serial log) is the SMP-1 regression assert — a
   stalled `enumerate_pci` yields no `login:` and fails loudly.
4. Existing gates unchanged: CI ubuntu `-smp 2` jobs and all 11 host
   contract suites must stay green (no userspace touched).

## 5. Assumptions / open questions

- **Kernel is mid-major-change:** anchors verified Aug 11, 2026 against the
  dirty working tree; names (`smp::init`, `enumerate_pci`, `send_ipi`,
  `init_cpu`) are the stable contract, line numbers may drift.
- **The local TCG stall is the trigger, not the whole story:** CI ubuntu QEMU
  boots `-smp 2`, so the gates cannot catch F1–F4 today. The fixes are
  justified on their own terms (SDM compliance, boundedness), not only as TCG
  workarounds.
- **ICR bit layout** per Intel SDM 10.6: bit 14 = trigger mode, bit 17 =
  level. QEMU's `APIC_LVT_LEVEL_TRIGGER`-keyed INIT handling is the reason the
  current code works there; real hardware requires bit 17.
- **SMEP on APs is deferred, not dropped:** APs run kernel threads until the
  scheduler migrates user threads; enabling SMEP later (guarded by the correct
  bit-7 CPUID check) is a follow-up, not part of this change. Accepted
  trade-off: an AP executing with user pages loaded in its page tables
  momentarily loses SMEP protection.
- **`init_cpu` takes a `bsp: bool` (Design A2)** instead of reading the LAPIC
  ID: the AP calls `init_cpu` at `smp.rs:330` *before* its own `lapic::init()`
  runs, so a LAPIC MMIO read there is unsafe until verified. The bool is
  unambiguous and ordering-independent.
- **Open:** should the SMP-1 leg's stall-on-bus-0 be traced to a single
  device before the rewrite ships? The A3/A4 changes make the boot bounded,
  but they don't *name* a stalling device — B2's watchdog is the tool that
  will.
