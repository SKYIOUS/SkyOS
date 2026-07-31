//! Inter-process communication wire protocol — canonical for ADE service calls.

use crate::errno::Error;
use alloc::vec::Vec;

/// Maximum payload size for one IPC message.
pub const MAX_IPC_MSG: usize = 4096;
/// Length-prefix header size (u32 LE).
pub const HEADER_LEN: usize = 4;

/// Canonical wire ids for ADE services (order must match ade ServiceId).
pub const SVC_CLIPBOARD: u8 = 0;
pub const SVC_NOTIFICATION: u8 = 1;
pub const SVC_LAUNCHER: u8 = 2;
pub const SVC_FILE_DIALOG: u8 = 3;
pub const SVC_SETTINGS: u8 = 4;
pub const SVC_SESSION: u8 = 5;
pub const SVC_WINDOW: u8 = 6;
pub const SVC_THEME: u8 = 7;
pub const SVC_POWER: u8 = 8;

/// Writes one complete frame (u32 LE length + payload) in a single write.
/// A single write becomes a single queued datagram at the peer; oversized
/// payloads are rejected rather than split.
pub fn write_frame(fd: i64, payload: &[u8]) -> Result<(), Error> {
    if payload.len() > MAX_IPC_MSG {
        return Err(Error::EINVAL);
    }
    let mut frame = alloc::vec![0u8; HEADER_LEN + payload.len()];
    frame[..HEADER_LEN].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    frame[HEADER_LEN..].copy_from_slice(payload);
    let n = crate::io::write(fd, &frame)?;
    if n != frame.len() {
        return Err(Error::EIO);
    }
    Ok(())
}

/// Reads one complete frame (header + payload) in a single read, returning the
/// payload length. Returns 0 on EOF (peer closed).
pub fn read_frame(fd: i64, buf: &mut [u8; MAX_IPC_MSG]) -> Result<usize, Error> {
    let mut frame = [0u8; HEADER_LEN + MAX_IPC_MSG];
    let n = crate::io::read(fd, &mut frame)?;
    if n == 0 {
        return Ok(0);
    }
    if n < HEADER_LEN {
        return Err(Error::EIO);
    }
    let len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    if len > MAX_IPC_MSG {
        return Err(Error::EINVAL);
    }
    if n != HEADER_LEN + len {
        return Err(Error::EIO);
    }
    buf[..len].copy_from_slice(&frame[HEADER_LEN..HEADER_LEN + len]);
    Ok(len)
}

/// request := u64 LE request_id | u8 service | u32 LE method_len | method | u32 LE args_len | args
pub fn encode_request(req_id: u64, service: u8, method: &[u8], args: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&req_id.to_le_bytes());
    out.push(service);
    out.extend_from_slice(&(method.len() as u32).to_le_bytes());
    out.extend_from_slice(method);
    out.extend_from_slice(&(args.len() as u32).to_le_bytes());
    out.extend_from_slice(args);
    out
}

pub fn decode_request(buf: &[u8]) -> Option<(u64, u8, Vec<u8>, Vec<u8>)> {
    if buf.len() < 13 {
        return None;
    }
    let req_id = u64::from_le_bytes(buf[0..8].try_into().ok()?);
    let service = buf[8];
    let mut pos = 9;
    let method_len = u32::from_le_bytes(buf[pos..pos + 4].try_into().ok()?) as usize;
    pos += 4;
    if pos + method_len + 4 > buf.len() {
        return None;
    }
    let method = buf[pos..pos + method_len].to_vec();
    pos += method_len;
    let args_len = u32::from_le_bytes(buf[pos..pos + 4].try_into().ok()?) as usize;
    pos += 4;
    if pos + args_len != buf.len() {
        return None;
    }
    let args = buf[pos..pos + args_len].to_vec();
    Some((req_id, service, method, args))
}

/// response := u64 LE request_id | u8 success | u32 LE data_len | data
pub fn encode_response(req_id: u64, success: bool, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&req_id.to_le_bytes());
    out.push(success as u8);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    out
}

pub fn decode_response(buf: &[u8]) -> Option<(u64, bool, Vec<u8>)> {
    if buf.len() < 13 {
        return None;
    }
    let req_id = u64::from_le_bytes(buf[0..8].try_into().ok()?);
    let success = buf[8] != 0;
    let data_len = u32::from_le_bytes(buf[9..13].try_into().ok()?) as usize;
    if 13 + data_len != buf.len() {
        return None;
    }
    Some((req_id, success, buf[13..13 + data_len].to_vec()))
}
