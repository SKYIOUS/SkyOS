# Memory System Calls

The memory syscalls manage virtual memory mappings.

## mmap (syscall 9)

```c
void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset);
```

Creates a new mapping in the virtual address space. Supports:
- Anonymous mappings (`MAP_ANONYMOUS`)
- File-backed mappings
- Shared (`MAP_SHARED`) and private (`MAP_PRIVATE`) mappings
- Fixed address mappings (`MAP_FIXED`)

## munmap (syscall 11)

```c
int munmap(void *addr, size_t length);
```

Unmaps previously mapped memory. The address must be page-aligned.

## mprotect (syscall 10)

```c
int mprotect(void *addr, size_t len, int prot);
```

Changes access protections for a memory region. Protection flags: `PROT_NONE`, `PROT_READ`,
`PROT_WRITE`, `PROT_EXEC`.

## brk (syscall 12)

```c
int brk(void *addr);
void *sbrk(intptr_t increment);
```

Changes the program break (end of the data segment). Used by `malloc()` for heap management.

## memfd_create (syscall 319)

```c
int memfd_create(const char *name, unsigned int flags);
```

Creates an anonymous file backed by RAM, returning a file descriptor.

## Shared Memory

```c
int shmget(key_t key, size_t size, int shmflg);   // syscall 29
void *shmat(int shmid, const void *shmaddr, int shmflg);   // syscall 30
int shmctl(int shmid, int cmd, struct shmid_ds *buf);   // syscall 31
int shmdt(const void *shmaddr);   // syscall 67
```

System V shared memory interface. `shmget` allocates, `shmat` attaches, `shmdt` detaches.
