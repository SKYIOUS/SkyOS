#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::println;

const BLOCK_SIZE: usize = 4096;
const SECTOR_SIZE: usize = 512;
const SECTORS_PER_BLOCK: u64 = 8;

const SKYFS_MAGIC: u64 = 0x315620534B59534B;
const SKYFS_VERSION: u32 = 1;

fn user_main() -> i32 {
    let dev_path = libsarga::args::get(1).unwrap_or("/dev/sda");

    let fd = match io::open(dev_path, 2) {
        Ok(f) => f,
        Err(e) => { println!("Error: cannot open {}: {}", dev_path, e); return 0; }
    };

    let total_blocks: u64 = 65536;
    let journal_blocks: u64 = 64;
    let bitmap_blocks: u64 = (total_blocks + BLOCK_SIZE as u64 * 8 - 1) / (BLOCK_SIZE as u64 * 8);
    let inode_count: u64 = total_blocks / 4;
    let inodes_per_block: u64 = BLOCK_SIZE as u64 / 256;
    let inode_blocks: u64 = (inode_count + inodes_per_block - 1) / inodes_per_block;

    let journal_start: u64 = 1;
    let bitmap_start: u64 = journal_start + journal_blocks;
    let inode_start: u64 = bitmap_start + bitmap_blocks;

    let superblock = SkyfsSuperblock {
        magic: SKYFS_MAGIC, version: SKYFS_VERSION, block_size: BLOCK_SIZE as u32,
        total_blocks, journal_start, journal_blocks,
        bitmap_start, bitmap_blocks, inode_start,
        inode_count, inode_blocks, root_inode: 1, state: 1,
    };

    let mut sb_buf = [0u8; BLOCK_SIZE];
    let sb_bytes = unsafe {
        core::slice::from_raw_parts(&superblock as *const SkyfsSuperblock as *const u8,
            core::mem::size_of::<SkyfsSuperblock>())
    };
    sb_buf[..sb_bytes.len()].copy_from_slice(sb_bytes);
    for i in 0..SECTORS_PER_BLOCK {
        let start = i as usize * SECTOR_SIZE;
        if io::write(fd, &sb_buf[start..start + SECTOR_SIZE]).is_err() {
            println!("Error writing superblock sector {}", i);
            return 0;
        }
    }

    let root_inode = SkyfsInode {
        mode: 0o040755, uid: 0, gid: 0, size: 0,
        atime: 0, mtime: 0, ctime: 0, links: 2, flags: 0,
        block_count: 0, extent_count: 0, data: [0u8; 256],
    };
    let inode_bytes = unsafe {
        core::slice::from_raw_parts(&root_inode as *const SkyfsInode as *const u8,
            core::mem::size_of::<SkyfsInode>())
    };

    // Skip to inode block (zero-fill journal and bitmap areas)
    let target_sector = inode_start * SECTORS_PER_BLOCK;
    let mut skip = target_sector * SECTOR_SIZE as u64;
    let zero_buf = [0u8; 512];
    while skip > 0 {
        let to_write = if skip > 512 { 512 } else { skip as usize };
        if io::write(fd, &zero_buf[..to_write]).is_err() { break; }
        skip -= to_write as u64;
    }

    // Write inode block with root inode at offset
    let inode_block_offset = (1 % inodes_per_block) as usize * 256;
    let mut inode_buf = [0u8; BLOCK_SIZE];
    inode_buf[inode_block_offset..inode_block_offset + inode_bytes.len()].copy_from_slice(inode_bytes);
    for i in 0..SECTORS_PER_BLOCK {
        let start = i as usize * SECTOR_SIZE;
        if io::write(fd, &inode_buf[start..start + SECTOR_SIZE]).is_err() {
            println!("Error writing inode block");
            return 0;
        }
    }

    io::close(fd).ok();
    println!("SkyFS filesystem created on {}: {} blocks, {} inodes", dev_path, total_blocks, inode_count);
    0
}

#[repr(C, packed)]
struct SkyfsSuperblock {
    magic: u64, version: u32, block_size: u32, total_blocks: u64,
    journal_start: u64, journal_blocks: u64, bitmap_start: u64,
    bitmap_blocks: u64, inode_start: u64, inode_count: u64,
    inode_blocks: u64, root_inode: u64, state: u8,
}

#[repr(C, packed)]
struct SkyfsInode {
    mode: u16, uid: u32, gid: u32, size: u64,
    atime: u64, mtime: u64, ctime: u64, links: u32, flags: u32,
    block_count: u64, extent_count: u32, data: [u8; 256],
}

sarga_main!(user_main);
