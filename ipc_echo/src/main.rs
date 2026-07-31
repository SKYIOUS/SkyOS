#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, io, sarga_main};

fn user_main() -> i32 {
    let mut ipc_fd = -1i64;
    let argc = args::argc();
    let mut i = 0;
    while i < argc {
        if args::get(i as usize) == Some("--ipc-fd") {
            if let Some(v) = args::get((i + 1) as usize) {
                if let Ok(n) = v.parse::<i64>() {
                    ipc_fd = n;
                }
            }
        }
        i += 1;
    }
    if ipc_fd < 0 {
        io::print_str("[ipc_echo] no --ipc-fd arg\n");
        return 1;
    }

    // Request a notification via the ADE security portal.
    let args_payload: &[u8] = b"IPC Echo\0Hello from ipc_echo via AF_UNIX socketpair\01\0";
    let req = libsarga::ipc::encode_request(1, libsarga::ipc::SVC_NOTIFICATION, b"notify", args_payload);
    if libsarga::ipc::write_frame(ipc_fd, &req).is_err() {
        io::print_str("[ipc_echo] send failed\n");
        return 1;
    }

    let mut buf = [0u8; libsarga::ipc::MAX_IPC_MSG];
    match libsarga::ipc::read_frame(ipc_fd, &mut buf) {
        Ok(0) => {
            io::print_str("[ipc_echo] connection closed by server\n");
            1
        }
        Ok(n) => match libsarga::ipc::decode_response(&buf[..n]) {
            Some((req_id, success, data)) => {
                io::print_str(&alloc::format!(
                    "[ipc_echo] response req={} success={} data_len={}\n",
                    req_id,
                    success,
                    data.len()
                ));
                if success { 0 } else { 1 }
            }
            None => {
                io::print_str("[ipc_echo] bad response frame\n");
                1
            }
        },
        Err(e) => {
            io::print_str(&alloc::format!("[ipc_echo] read failed: {}\n", e));
            1
        }
    }
}

sarga_main!(user_main);
