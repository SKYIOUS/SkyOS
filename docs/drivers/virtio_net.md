# VirtIO Network Driver

The VirtIO network driver provides networking for virtualized environments (QEMU, VirtualBox, KVM).

## VirtIO Overview

VirtIO is a paravirtualized I/O framework. The guest OS communicates with the host through virtqueues: circular buffers of descriptors shared between guest and host.

```rust
pub struct VirtIOQueue {
    descriptors: VirtqDesc,   // descriptor ring
    available: VirtqAvail,
    used: VirtqUsed,
    // ...
}
```

## Driver Implementation

The driver lives at `kernel/kernel/src/drivers/net/virtio.rs`. Key types:

```rust
pub struct VirtIONet { /* virtqueue rings + device regs */ }
impl VirtIONet {
    pub fn new(base_addr: u16) -> Self;                 // MMIO BAR base
    pub fn mac_address(&self) -> [u8; 6];
    pub fn send_packet(&mut self, data: &[u8]);
    pub fn receive_packet(&mut self) -> Option<Vec<u8>>;
}
pub struct VirtIONetDevice;  // impl smoltcp Device trait (RxToken/TxToken)
```

The driver uses the **legacy I/O port transport** (`x86_64::Port` at the device's base I/O address) and adapts to smoltcp's `Device` trait for the net feature. Vendor/device IDs `0x1AF4:0x1000` identify VirtIO net devices during PCI enumeration.

## Initialization

1. `VirtIONet::new(io_base)` reads the VirtIO registers at the device's base I/O port
2. `VirtIONetDevice` wraps the driver for smoltcp's `Device` trait
3. smoltcp drives it through `RxToken`/`TxToken`

## Packet Transmission

To transmit a packet:
1. Get a free descriptor from the TX virtqueue
2. Fill the descriptor with the packet data address and length
3. Add the descriptor to the available ring
4. Notify the device by writing to the queue notify register
5. On completion interrupt, reclaim used descriptors

## Packet Reception

Receive buffers are pre-allocated and placed in the RX virtqueue. When a packet arrives, the device writes data into a buffer and adds the descriptor to the used ring. The driver's interrupt handler processes used buffers and replaces them with fresh ones.
