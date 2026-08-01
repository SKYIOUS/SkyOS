# Filesystem System Calls

The filesystem syscalls provide directory and file manipulation operations. Numbers follow the
Linux x86_64 ABI (see `docs/syscalls/index.md` for the full table).

## stat / fstat / lstat (syscalls 4-6)

```c
int stat(const char *pathname, struct stat *statbuf);
int fstat(int fd, struct stat *statbuf);
int lstat(const char *pathname, struct stat *statbuf);
```

Retrieves file status information. `lstat` returns information about the symbolic link itself, not
the target.

## mkdir (syscall 83)

```c
int mkdir(const char *pathname, mode_t mode);
```

Creates a new directory with the specified permissions. (Directories are removed with `unlink`;
there is no separate `rmdir` syscall.)

## unlink (syscall 87)

```c
int unlink(const char *pathname);
```

Removes a name from the filesystem. The file data is freed when no more references exist.

## link (syscall 86)

```c
int link(const char *oldpath, const char *newpath);
```

Creates a hard link to an existing file. Both names refer to the same inode.

## symlink (syscall 88)

```c
int symlink(const char *target, const char *linkpath);
```

Creates a symbolic link containing the string `target`.

## readlink (syscall 89)

```c
ssize_t readlink(const char *pathname, char *buf, size_t bufsiz);
```

Reads the target of a symbolic link into `buf`.

## rename (syscall 82)

```c
int rename(const char *oldpath, const char *newpath);
```

Renames a file or directory, potentially moving it between directories on the same filesystem.

## truncate / ftruncate (syscalls 76-77)

```c
int truncate(const char *path, off_t length);
int ftruncate(int fd, off_t length);
```

Truncates or extends a file to the specified length.

## chmod / fchmod / chown / fchown (syscalls 90-93)

```c
int chmod(const char *pathname, mode_t mode);
int fchmod(int fd, mode_t mode);
int chown(const char *pathname, uid_t owner, gid_t group);
int fchown(int fd, uid_t owner, gid_t group);
```

Changes file permissions and ownership.

## getdents64 (syscall 217)

```c
int getdents64(unsigned int fd, struct linux_dirent64 *dirp, unsigned int count);
```

Reads directory entries from a directory file descriptor.

## mount / umount2 (syscalls 165/167)

```c
int mount(const char *source, const char *target, const char *fstype, unsigned long flags, const void *data);
int umount2(const char *target, int flags);
```

Mounts and unmounts filesystems. Mounting requires the `CAP_SYS_ADMIN` capability.
