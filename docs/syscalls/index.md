# Complete Syscall Table

This page lists all system calls implemented in SkyOS, organized by number.
Numbers follow the Linux x86_64 ABI where a POSIX equivalent exists; kernel-specific extensions
live in dedicated ranges (100–127 GUI/audio, 300–352 Vahi/AI/credentials/signals, 380–401 object
manager/hashing, 425–426 io_uring).

## Syscall Convention

- RAX: syscall number
- RDI, RSI, RDX, R10, R8, R9: arguments
- Return value in RAX (negative = errno on error)

## Syscall Table

| # | Name | Description |
|---|------|-------------|
| 0 | read | Read from file descriptor |
| 1 | write | Write to file descriptor |
| 2 | open | Open a file |
| 3 | close | Close a file descriptor |
| 4 | stat | Get file status |
| 5 | fstat | Get file descriptor status |
| 6 | lstat | Get symbolic link status |
| 7 | poll | Wait for I/O events |
| 8 | lseek | Reposition file offset |
| 9 | mmap | Map memory |
| 10 | mprotect | Set memory protection |
| 11 | munmap | Unmap memory |
| 12 | brk | Change data segment size |
| 13 | rt_sigaction | Set signal handler |
| 15 | rt_sigreturn | Return from signal handler |
| 16 | ioctl | Device control |
| 21 | access | Check file permissions |
| 22 | pipe | Create pipe |
| 23 | select | Synchronous I/O multiplexing |
| 24 | sched_yield | Yield processor |
| 29 | shmget | Allocate shared memory |
| 30 | shmat | Attach shared memory |
| 31 | shmctl | Control shared memory |
| 32 | dup | Duplicate file descriptor |
| 33 | dup2 | Duplicate to specific fd |
| 34 | pause | Wait for signal |
| 35 | nanosleep | High-resolution sleep |
| 36 | sync | Flush filesystems to disk |
| 39 | getpid | Get process ID |
| 40 | sendfile | Transfer data between fds |
| 41 | socket | Create socket |
| 42 | connect | Connect socket |
| 43 | accept | Accept connection |
| 44 | sendto | Send message on socket |
| 45 | recvfrom | Receive message from socket |
| 46 | sendmsg | Send message with ancillary data |
| 47 | recvmsg | Receive message with ancillary data |
| 49 | bind | Bind socket to address |
| 50 | listen | Listen on socket |
| 51 | getsockname | Get socket name |
| 52 | getpeername | Get peer name |
| 53 | socketpair | Create connected socket pair |
| 54 | setsockopt | Set socket options |
| 55 | getsockopt | Get socket options |
| 56 | clone | Create child process/thread |
| 57 | fork | Create child process |
| 59 | execve | Execute program |
| 60 | exit | Terminate process |
| 61 | wait4 | Wait for process |
| 62 | kill | Send signal |
| 63 | uname | Get system information |
| 67 | shmdt | Detach shared memory |
| 72 | fcntl | Manipulate file descriptor |
| 76 | truncate | Truncate file |
| 77 | ftruncate | Truncate file by fd |
| 79 | getcwd | Get current working directory |
| 80 | chdir | Change directory |
| 82 | rename | Rename file |
| 83 | mkdir | Create directory |
| 86 | link | Create hard link |
| 87 | unlink | Remove file |
| 88 | symlink | Create symbolic link |
| 89 | readlink | Read symbolic link target |
| 90 | chmod | Change file mode |
| 91 | fchmod | Change file mode by fd |
| 92 | chown | Change file owner |
| 93 | fchown | Change file owner by fd |
| 95 | umask | Set file mode creation mask |
| 97 | getrlimit | Get resource limits |
| 98 | setrlimit | Set resource limits |
| 100 | gui_create_window | Create GUI window |
| 101 | gui_get_buffer | Get window content size |
| 102 | gui_flush | Flush window updates |
| 103 | gui_map_buffer | Map window framebuffer |
| 104 | beep | Emit PC-speaker tone |
| 105 | gui_get_key | Pop next key event |
| 110 | getppid | Get parent process ID |
| 111 | getpgrp | Get process group |
| 112 | setsid | Create session |
| 115 | getgroups | Get supplementary group IDs |
| 116 | setgroups | Set supplementary group IDs |
| 118 | getresuid | Get real/effective/saved UID |
| 119 | setresuid | Set real/effective/saved UID |
| 120 | gui_get_mouse | Get mouse state |
| 121 | gui_set_title | Set window title |
| 122 | gui_destroy_window | Destroy window |
| 123 | gui_resize_window | Resize window |
| 124 | gui_move_window | Move window |
| 125 | clipboard | Read/write compositor clipboard |
| 126 | notify | Queue desktop notification |
| 127 | mkfs | Create filesystem on device |
| 131 | sigaltstack | Set alternate signal stack |
| 137 | statfs | Get filesystem statistics |
| 144 | sched_setattr | Set scheduling attributes |
| 145 | sched_getattr | Get scheduling attributes |
| 157 | setpgid | Set process group |
| 158 | arch_prctl | Architecture-specific setup |
| 165 | mount | Mount filesystem |
| 167 | umount2 | Unmount filesystem |
| 169 | reboot | Reboot the system |
| 200 | resolve | Resolve path |
| 201 | korlang | KorLang interpreter |
| 202 | futex | Fast userspace mutex |
| 203 | sysinfo | Get system statistics |
| 210 | openpty | Open pseudo-terminal pair |
| 217 | getdents64 | Get directory entries (64-bit) |
| 218 | set_tid_address | Set TID address |
| 222 | timer_create | Create POSIX timer |
| 223 | timer_settime | Arm POSIX timer |
| 224 | timer_gettime | Get POSIX timer |
| 225 | timer_getoverrun | Get timer overrun count |
| 226 | timer_delete | Delete POSIX timer |
| 228 | clock_gettime | Get clock time |
| 231 | exit_group | Exit all threads |
| 257 | openat | Open file relative to dirfd |
| 258 | mkdirat | Create directory relative to dirfd |
| 262 | fstatat | Get file status relative to dirfd |
| 263 | unlinkat | Remove file relative to dirfd |
| 264 | renameat | Rename relative to dirfds |
| 265 | linkat | Create hard link relative to dirfd |
| 266 | symlinkat | Create symlink relative to dirfd |
| 267 | readlinkat | Read symlink relative to dirfd |
| 269 | faccessat | Check access relative to dirfd |
| 280 | utimensat | Change file timestamps |
| 282 | signalfd | Create signal file descriptor |
| 284 | eventfd | Create event file descriptor |
| 285 | fallocate | Preallocate file space |
| 289 | signalfd4 | Create signal fd (flags) |
| 290 | eventfd2 | Create event fd (flags) |
| 300 | vahiai | Vahiai rule-engine dispatch |
| 301 | getuid | Get user ID |
| 302 | getgid | Get group ID |
| 303 | setuid | Set user ID |
| 304 | setgid | Set group ID |
| 305 | geteuid | Get effective user ID |
| 306 | getegid | Get effective group ID |
| 307 | capget | Get capabilities |
| 308 | capset | Set capabilities |
| 309 | sigprocmask | Examine/change signal mask |
| 310 | ash_register | Register ASH handler (feature `ash`) |
| 311 | ash_unregister | Unregister ASH handler (feature `ash`) |
| 312 | ash_stats | Get ASH statistics (feature `ash`) |
| 313 | ash_control | Control ASH (feature `ash`) |
| 314 | getresgid | Get real/effective/saved GID |
| 315 | setresgid | Set real/effective/saved GID |
| 319 | memfd_create | Create anonymous file |
| 321 | bpf | BPF program control |
| 326 | swapon | Enable swap device |
| 327 | swapoff | Disable swap device |
| 330 | getpgid | Get process group by pid |
| 331 | getsid | Get session ID |
| 332 | prlimit64 | Read/set process limits |
| 340 | vm_create | Create VM (feature `hypervisor`) |
| 341 | vm_destroy | Destroy VM (feature `hypervisor`) |
| 342 | vm_start | Start VM (feature `hypervisor`) |
| 343 | vm_stop | Stop VM (feature `hypervisor`) |
| 344 | vm_pause | Pause VM (feature `hypervisor`) |
| 345 | vm_resume | Resume VM (feature `hypervisor`) |
| 346 | vm_load_kernel | Load kernel into VM (feature `hypervisor`) |
| 347 | vm_get_info | Get VM info (feature `hypervisor`) |
| 348 | vm_set_memory | Set VM memory (feature `hypervisor`) |
| 349 | vm_inject_irq | Inject IRQ into VM (feature `hypervisor`) |
| 350 | getitimer | Get interval timer |
| 351 | setitimer | Set interval timer |
| 352 | times | Get process times |
| 380 | objmgr_enum | Enumerate object manager |
| 381 | objmgr_audit | Audit object manager |
| 400 | drmctl | DRM device control |
| 401 | hash | Kernel hash computation |
| 425 | io_uring_setup | Set up io_uring |
| 426 | io_uring_enter | Enter io_uring |
