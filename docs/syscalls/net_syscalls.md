# Networking System Calls

The networking syscalls provide socket and network operations, backed by smoltcp. All socket
syscalls below are implemented (the `net` kernel feature) — see `docs/socket-api.md` for details.

## socket (syscall 41)

```c
int socket(int domain, int type, int protocol);
```

Creates a socket endpoint for communication. Returns a file descriptor on success.

**Domains**: `AF_INET` (2), `AF_INET6` (10), `AF_UNIX` (1, used for socketpair IPC)

**Types**: `SOCK_STREAM`, `SOCK_DGRAM`, `SOCK_RAW` (requires `CAP_NET_RAW`), `SOCK_NONBLOCK`

## bind (syscall 49)

```c
int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
```

Binds a socket to a local address.

## listen (syscall 50)

```c
int listen(int sockfd, int backlog);
```

Marks a socket as passive (listening for incoming connections). `backlog` limits the pending
connection queue.

## accept (syscall 43)

```c
int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
```

Accepts an incoming connection on a listening socket. Returns a new file descriptor for the
connection. Returns `EINTR` if interrupted by a signal before a connection arrives.

## connect (syscall 42)

```c
int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
```

Connects a socket to a remote address.

## sendto / recvfrom (syscalls 44-45)

```c
ssize_t sendto(int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen);
ssize_t recvfrom(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen);
```

Send and receive datagrams on connectionless sockets.

## sendmsg / recvmsg (syscalls 46-47)

Advanced message send/receive with scatter-gather I/O and ancillary data support.

## socketpair (syscall 53)

```c
int socketpair(int domain, int type, int protocol, int sv[2]);
```

Creates an unnamed pair of connected sockets (used for `AF_UNIX` IPC, e.g. the ADE launcher).

## setsockopt / getsockopt (syscalls 54-55)

```c
int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen);
int getsockopt(int sockfd, int level, int optname, void *optval, socklen_t *optlen);
```

Set and get socket options. `SOL_SOCKET` `SO_RCVTIMEO`/`SO_SNDTIMEO` are accepted but sockets are
non-blocking; `IPPROTO_TCP` `TCP_NODELAY` is honored.

## getsockname / getpeername (syscalls 51-52)

```c
int getsockname(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
int getpeername(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
```

Get local and remote socket addresses.
