# Network Stack Architecture

The SkyOS network stack is built on the **smoltcp** library (feature `net`). There is no in-house TCP/IP implementation.

## Architecture

- **Drivers** (`kernel/kernel/src/drivers/net/`): e1000 (`e1000.rs`) and VirtIO net (`virtio.rs`) adapt the NIC to smoltcp's `phy::Device` trait via `RxToken`/`TxToken` implementations.
- **Interfaces**: The `net` module registers the driver device as the interface (IP configuration, MAC address).
- **Protocols**: smoltcp provides ARP, IPv4/IPv6, TCP, UDP, ICMP, and DHCP (via the `socket-dhcpv4` feature for the management interface).
- **Sockets**: The kernel socket API (syscalls 41–54) maps onto smoltcp socket types (TCP, UDP, ICMP). Sockets are tracked in the process fd table as `FileDescriptor::Socket`. See `docs/socket-api.md`.

## Syscall Surface

- `socket`(41), `bind`(49), `connect`(42), `listen`(50), `accept`(43), `sendto`(44), `recvfrom`(45), `setsockopt`(54)
- AF_INET=2, AF_INET6=10; `SOCK_RAW` gated on `CAP_NET_RAW`
- Non-blocking semantics: timeouts via `SO_RCVTIMEO`/`SO_SNDTIMEO` are accepted but unused — sockets are non-blocking, with read/write returning `EAGAIN`.

## Packet Buffering

Buffers are smoltcp-owned (`smoltcp::phy::Device` receive/transmit tokens). There is no kernel `mbuf`-style `NetBuf` structure.

## Future Plans

- WireGuard VPN integration
- TCP offload engine (TOE) for capable NICs
- Network stack virtualization for container networking
