# Kernel Architecture Overview

SkyOS is a **monolithic** kernel. The kernel is a single privileged executable that provides core services (memory management, scheduling, IPC) and also runs traditional OS services (filesystems, drivers, networking) **in kernel space**. There are no userspace FS/driver/net tasks; every filesystem and driver lives in `kernel/src`.

## Design Philosophy

The architecture prioritizes three core principles:

- **Safety**: By leveraging Rust's type system and ownership model, we eliminate entire classes of bugs at compile time. The kernel is written almost entirely in safe Rust, with `unsafe` blocks confined to hardware interaction and low-level initialization.
- **Asynchrony**: An async executor (`task/executor.rs`) runs in a dedicated kernel thread for event-driven work; the scheduler itself is preemptive (LAPIC timer).
- **Modularity**: Each subsystem is isolated behind well-defined interfaces. Filesystems implement the `FileSystem`/`VfsNode` traits; drivers implement `BlockDevice`/`NicDevice` etc.

## Kernel Structure

The kernel is organized into these major layers:

1. **Arch Layer** (`src/arch/`): Platform-specific code including GDT, IDT, paging, and CPU initialization.
2. **Core Layer** (`src/task/`, `src/memory/`, `src/hal/`): Kernel primitives including scheduling, memory management, and synchronization.
3. **Services Layer** (`src/syscalls/`): System call dispatch; IPC is provided in-kernel via AF_UNIX sockets, pipes, futexes, eventfd, and PTYs.
4. **Driver Framework** (`src/drivers/`): Hardware abstraction and device driver infrastructure.
5. **Filesystem Layer** (`src/vfs/`): Virtual file system interface and implementations (ramfs, devfs, ctlfs, pipe, tarfs, skyfs, ext2, ext4, fat).

## Monolithic (not hybrid microkernel)

SkyOS is a monolithic kernel: scheduling, memory, IPC, filesystems, network stacks, and drivers all run in kernel space. This provides lower overhead than a microkernel at the cost of isolation (a driver bug can crash the kernel). There is no userspace driver/FS model.
