# Network Stack Testing

The network stack (smoltcp, `net` feature) is exercised through QEMU with an e1000 NIC and user-mode networking.

## QEMU Boot Tests

`tests/qemu_boot.sh` and `tests/qemu_integration_test.sh` boot the ISO in QEMU with `-device e1000,netdev=net0 -netdev user,id=net0`. A boot reaching the `login:` prompt implies the NIC initialized and DHCP/socket setup ran (the management interface uses DHCP).

## Protocol Coverage

smoltcp itself is third-party and unit-tested upstream. Kernel-level protocol behavior (ARP, TCP, UDP, ICMP, DHCP) is verified by booting with the `net` feature and observing the interface come up; there is no dedicated kernel packet test suite.

## Commands

```bash
./tests/qemu_boot.sh
./tests/qemu_integration_test.sh
```

There is no `cargo test --test bench network_throughput` or packet-capture/namespace harness.
