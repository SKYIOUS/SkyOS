#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::{args, io, println, sarga_main};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn read_whole_fd(fd: i64) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    out
}

fn gzip_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 32);
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0x03]);
    out.extend_from_slice(&compress_to_vec(data, 6));
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&((data.len() as u32).to_le_bytes()));
    out
}

fn gzip_decompress(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b {
        return Err("not in gzip format");
    }
    let mut off = 10usize;
    let flags = data[3];
    if flags & 0x04 != 0 {
        // FEXTRA
        if off + 2 > data.len() {
            return Err("corrupt gzip header");
        }
        let xlen = u16::from_le_bytes([data[off], data[off + 1]]) as usize;
        off += 2 + xlen;
    }
    if flags & 0x08 != 0 {
        // FNAME
        while off < data.len() && data[off] != 0 {
            off += 1;
        }
        off += 1;
    }
    if flags & 0x10 != 0 {
        // FCOMMENT
        while off < data.len() && data[off] != 0 {
            off += 1;
        }
        off += 1;
    }
    if flags & 0x02 != 0 {
        // FHCRC
        off += 2;
    }
    if off + 8 > data.len() {
        return Err("truncated gzip file");
    }
    decompress_to_vec(&data[off..data.len() - 8]).map_err(|_| "decompression failed")
}

fn user_main() -> i32 {
    let mut decompress = false;
    let mut file: Option<&str> = None;
    for i in 1..args::argc() {
        if let Some(a) = args::get(i as usize) {
            if a == "-d" || a == "--decompress" || a == "-dc" || a == "-cd" {
                decompress = true;
            } else if a.starts_with('-') && a.len() > 1 && a != "--" {
                // ignore -c (stdout), -k, -f for now
            } else if a == "--" {
                continue;
            } else {
                file = Some(a);
            }
        }
    }

    let data = match file {
        Some(path) => match io::open(path, 0) {
            Ok(fd) => {
                let d = read_whole_fd(fd);
                let _ = io::close(fd);
                d
            }
            Err(_) => {
                println!("gzip: {}: No such file", path);
                return 1;
            }
        },
        None => read_whole_fd(0),
    };

    let result = if decompress {
        gzip_decompress(&data)
    } else {
        Ok(gzip_compress(&data))
    };

    match result {
        Ok(out) => {
            let _ = io::write(1, &out);
            0
        }
        Err(msg) => {
            println!("gzip: {}", msg);
            1
        }
    }
}

sarga_main!(user_main);
