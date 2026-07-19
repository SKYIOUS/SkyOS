#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{fs, io, sarga_main, toml::TomlDocument};

fn file_exists(path: &str) -> bool {
    let fd = io::open(path, 0);
    if let Ok(fd) = fd {
        let _ = io::close(fd);
        true
    } else {
        false
    }
}

fn copy_file(src: &str, dst: &str) -> bool {
    let src_fd = match io::open(src, 0) {
        Ok(fd) => fd,
        Err(_) => return false,
    };
    let dst_fd = match io::open(dst, 0x41) {
        // O_WRONLY | O_CREAT
        Ok(fd) => fd,
        Err(_) => {
            let _ = io::close(src_fd);
            return false;
        }
    };
    let mut buf = [0u8; 8192];
    loop {
        match io::read(src_fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let _ = io::write_all(dst_fd, &buf[..n]);
            }
            Err(_) => break,
        }
    }
    let _ = io::close(src_fd);
    let _ = io::close(dst_fd);
    true
}

fn user_main() -> i32 {
    io::print_str("[skyd-update] daemon starting\n");

    // Check if an update manifest is staged
    if file_exists("/tmp/update.toml") {
        io::print_str("[skyd-update] staged update manifest found, applying...\n");

        // Read and parse the manifest
        match fs::read_to_string("/tmp/update.toml") {
            Ok(manifest_str) => {
                match TomlDocument::parse(&manifest_str) {
                    Ok(doc) => {
                        // Get the files array from the manifest
                        let files = doc.get_tables("files");

                        if files.is_empty() {
                            io::print_str("[skyd-update] no files in manifest\n");
                        } else {
                            let mut success_count = 0u32;
                            let mut fail_count = 0u32;

                            for file_table in files {
                                let path = file_table.iter().find(|(k, _)| k == "path").and_then(
                                    |(_, v)| {
                                        if let libsarga::toml::TomlValue::String(s) = v {
                                            Some(s.as_str())
                                        } else {
                                            None
                                        }
                                    },
                                );

                                let source = file_table
                                    .iter()
                                    .find(|(k, _)| k == "source")
                                    .and_then(|(_, v)| {
                                        if let libsarga::toml::TomlValue::String(s) = v {
                                            Some(s.as_str())
                                        } else {
                                            None
                                        }
                                    });

                                if let (Some(dest_path), Some(src_path)) = (path, source) {
                                    let staged_path = &alloc::format!("/tmp/{}", src_path);

                                    if file_exists(staged_path) {
                                        io::print_str(&alloc::format!(
                                            "[skyd-update] updating {}\n",
                                            dest_path
                                        ));

                                        // Ensure destination directory exists
                                        if let Some(parent_dir) = get_parent_dir(dest_path) {
                                            let _ = io::mkdir(&parent_dir, 0o755);
                                        }

                                        if copy_file(staged_path, dest_path) {
                                            success_count += 1;
                                        } else {
                                            io::print_str(&alloc::format!(
                                                "[skyd-update] failed to copy {}\n",
                                                dest_path
                                            ));
                                            fail_count += 1;
                                        }
                                    } else {
                                        io::print_str(&alloc::format!(
                                            "[skyd-update] staged file not found: {}\n",
                                            staged_path
                                        ));
                                        fail_count += 1;
                                    }
                                }
                            }

                            io::print_str(&alloc::format!(
                                "[skyd-update] update complete: {} succeeded, {} failed\n",
                                success_count,
                                fail_count
                            ));
                        }
                    }
                    Err(_) => {
                        io::print_str("[skyd-update] failed to parse manifest\n");
                    }
                }
            }
            Err(_) => {
                io::print_str("[skyd-update] failed to read manifest\n");
            }
        }

        // Cleanup manifest
        let _ = io::unlink("/tmp/update.toml");
    } else {
        io::print_str("[skyd-update] no staged update found\n");
    }

    0
}

/// Get parent directory of a path
fn get_parent_dir(path: &str) -> Option<alloc::string::String> {
    if let Some(last_slash) = path.rfind('/') {
        if last_slash == 0 {
            Some(alloc::string::String::from("/"))
        } else {
            Some(alloc::string::String::from(&path[..last_slash]))
        }
    } else {
        None
    }
}

sarga_main!(user_main);
