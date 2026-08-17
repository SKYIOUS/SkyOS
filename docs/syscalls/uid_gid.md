# User and Group System Calls

The user/group syscalls manage process credentials and permissions. SkyOS implements full POSIX
credentials (real, effective, saved, and fsuid/fsgid) plus a Linux-compatible capability model.
Credential syscalls live in the 300+ range.

## getuid / geteuid (syscalls 301/305)

```c
uid_t getuid(void);
uid_t geteuid(void);
```

Returns the real user ID and effective user ID of the calling process. The real UID identifies the
owner of the process; the effective UID is used for permission checks.

## getgid / getegid (syscalls 302/306)

```c
gid_t getgid(void);
gid_t getegid(void);
```

Returns the real group ID and effective group ID of the calling process.

## setuid / setgid (syscalls 303/304)

```c
int setuid(uid_t uid);
int setgid(gid_t gid);
```

Sets the effective user/group ID of the calling process. If the caller has the appropriate
capability, the real, effective, and saved IDs are all set; otherwise only the effective ID can be
set to the real or saved ID.

## getresuid / setresuid (syscalls 118/119)

```c
int getresuid(uid_t *ruid, uid_t *euid, uid_t *suid);
int setresuid(uid_t ruid, uid_t euid, uid_t suid);
```

Get or set the real, effective, and saved user IDs.

## getresgid / setresgid (syscalls 314/315)

```c
int getresgid(gid_t *rgid, gid_t *egid, gid_t *sgid);
int setresgid(gid_t rgid, gid_t egid, gid_t sgid);
```

Get or set the real, effective, and saved group IDs.

## getgroups / setgroups (syscalls 115/116)

```c
int getgroups(int size, gid_t list[]);
int setgroups(size_t size, const gid_t *list);
```

Returns or sets the list of supplementary group IDs for the calling process. If `size` is 0,
`getgroups` returns the number of groups without modifying `list`.

## Process Credentials

Each process has:
- **Real UID/GID**: The actual owner of the process
- **Effective UID/GID**: Used for access control checks
- **Saved set-user-ID**: Allows swapping between real and effective UID
- **Supplementary groups**: Additional group memberships

## Capabilities

```c
int capget(struct cap_header *header, struct cap_data *data);   // syscall 307
int capset(struct cap_header *header, const struct cap_data *data);   // syscall 308
```

SkyOS uses a Linux-compatible capability bitmask in addition to UID/GID. Notable positions:
`CAP_NET_RAW` (13), `CAP_SYS_ADMIN` (21), `CAP_KILL` (5), `CAP_SETPCAP` (8). See
`docs/security/overview.md` for the full model.
