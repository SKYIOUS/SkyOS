//! Explorer — file manager application.
#![allow(dead_code)]

use crate::vfs::VfsEntry;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::gui::Window;
use libsarga::theme::Theme;

/// Undo log entry for file operations
pub(crate) struct FileOpLog {
    pub op_type: u8, // 0=delete, 1=rename/move, 2=create
    pub path: String,
    pub old_path: String,
}

#[derive(Clone)]
pub(crate) struct ExplorerTab {
    pub path: String,
    pub entries: Vec<VfsEntry>,
    pub sort_by: u8, // 0=name,1=size,2=date,3=type
    pub sort_desc: bool,
    pub view_mode: u8, // 0=list,1=grid,2=icon,3=details
    pub scroll: u32,
    pub sel_idx: Option<usize>,
}

pub(crate) struct ExplorerState {
    pub id: u32,
    pub tabs: Vec<ExplorerTab>,
    pub active_tab: usize,
    pub history: Vec<String>,
    pub history_idx: i32,
    pub path: String,
    pub split: bool,
    pub split_tab: Option<ExplorerTab>,
    pub ops: Vec<FileOpLog>,
    pub search_query: String,
    pub search_results: Vec<VfsEntry>,
    pub search_active: bool,
    pub favorites: Vec<String>,
    pub show_preview: bool,
    pub preview_content: String,
}

impl ExplorerState {
    pub fn new(id: u32, start_path: &str) -> Self {
        let path = String::from(start_path);
        let mut history = Vec::new();
        history.push(path.clone());
        let tab = ExplorerTab {
            path: path.clone(),
            entries: Vec::new(),
            sort_by: 0,
            sort_desc: false,
            view_mode: 0,
            scroll: 0,
            sel_idx: None,
        };
        ExplorerState {
            id,
            tabs: alloc::vec![tab],
            active_tab: 0,
            history,
            history_idx: 0,
            path,
            split: false,
            split_tab: None,
            ops: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_active: false,
            favorites: Vec::new(),
            show_preview: false,
            preview_content: String::new(),
        }
    }

    pub fn active(&mut self) -> &mut ExplorerTab {
        &mut self.tabs[self.active_tab]
    }

    pub fn active_ref(&self) -> &ExplorerTab {
        &self.tabs[self.active_tab]
    }

    pub fn refresh(&mut self) {
        let path = self.tabs[self.active_tab].path.clone();
        let mut entries = crate::vfs::VfsContext::list_dir_static(&path);
        sort_entries(
            &mut entries,
            self.tabs[self.active_tab].sort_by,
            self.tabs[self.active_tab].sort_desc,
        );
        self.tabs[self.active_tab].entries = entries;
        self.tabs[self.active_tab].sel_idx = None;
    }

    pub fn set_sort(&mut self, by: u8) {
        let tab = &mut self.tabs[self.active_tab];
        if tab.sort_by == by {
            tab.sort_desc = !tab.sort_desc;
        } else {
            tab.sort_by = by;
            tab.sort_desc = false;
        }
        let desc = tab.sort_desc;
        sort_entries(&mut tab.entries, by, desc);
    }

    pub fn navigate(&mut self, path: &str) {
        let p = String::from(path);
        self.path = p.clone();
        self.tabs[self.active_tab].path = p.clone();
        self.history.truncate((self.history_idx + 1) as usize);
        self.history.push(p);
        self.history_idx = self.history.len() as i32 - 1;
        self.refresh();
    }

    pub fn go_back(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            let p = self.history[self.history_idx as usize].clone();
            self.path = p.clone();
            self.tabs[self.active_tab].path = p;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if (self.history_idx as usize) < self.history.len() - 1 {
            self.history_idx += 1;
            let p = self.history[self.history_idx as usize].clone();
            self.path = p.clone();
            self.tabs[self.active_tab].path = p;
            self.refresh();
        }
    }

    pub fn go_up(&mut self) {
        let cur = &self.tabs[self.active_tab].path;
        if cur == "/" {
            return;
        }
        let parent = if cur.ends_with('/') {
            let trimmed = cur.trim_end_matches('/');
            let slash = trimmed.rfind('/').unwrap_or(0);
            if slash == 0 {
                String::from("/")
            } else {
                String::from(&trimmed[..slash])
            }
        } else {
            let slash = cur.rfind('/').unwrap_or(0);
            if slash == 0 {
                String::from("/")
            } else {
                String::from(&cur[..slash])
            }
        };
        self.navigate(&parent);
    }

    pub fn home(&mut self) {
        self.navigate("/home");
    }

    pub fn enter_dir(&mut self, path: &str) {
        self.navigate(path);
    }

    pub fn new_tab(&mut self, path: &str) {
        let tab = ExplorerTab {
            path: String::from(path),
            entries: Vec::new(),
            sort_by: 0,
            sort_desc: false,
            view_mode: 0,
            scroll: 0,
            sel_idx: None,
        };
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.refresh();
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.tabs.remove(idx);
        if self.active_tab >= idx && self.active_tab > 0 {
            self.active_tab -= 1;
        }
    }

    // ---- File Operations ----

    pub fn copy_selected(&mut self, dest: &str) {
        let tab = self.active_tab;
        let path = match self.tabs[tab]
            .sel_idx
            .map(|i| self.tabs[tab].entries[i].path.clone())
        {
            Some(p) => p,
            None => return,
        };
        let name = match self.tabs[tab]
            .sel_idx
            .map(|i| self.tabs[tab].entries[i].name.clone())
        {
            Some(n) => n,
            None => return,
        };
        let dest_path = if dest.ends_with('/') {
            alloc::format!("{}{}", dest, name)
        } else {
            alloc::format!("{}/{}", dest, name)
        };
        let _ = copy_file(&path, &dest_path);
        self.refresh();
    }

    pub fn move_selected(&mut self, dest: &str) {
        let tab = self.active_tab;
        let path = match self.tabs[tab]
            .sel_idx
            .map(|i| self.tabs[tab].entries[i].path.clone())
        {
            Some(p) => p,
            None => return,
        };
        let name = match self.tabs[tab]
            .sel_idx
            .map(|i| self.tabs[tab].entries[i].name.clone())
        {
            Some(n) => n,
            None => return,
        };
        let dest_path = if dest.ends_with('/') {
            alloc::format!("{}{}", dest, name)
        } else {
            alloc::format!("{}/{}", dest, name)
        };
        if rename_file(&path, &dest_path).is_ok() {
            self.ops.push(FileOpLog {
                op_type: 1,
                path: dest_path,
                old_path: path,
            });
        }
        self.refresh();
    }

    pub fn rename_selected(&mut self, new_name: &str) {
        let tab = self.active_tab;
        let idx = match self.tabs[tab].sel_idx {
            Some(i) => i,
            None => return,
        };
        let old_path = self.tabs[tab].entries[idx].path.clone();
        let parent = match old_path.rfind('/') {
            Some(s) => &old_path[..s + 1],
            None => return,
        };
        let new_path = alloc::format!("{}{}", parent, new_name);
        if rename_file(&old_path, &new_path).is_ok() {
            self.ops.push(FileOpLog {
                op_type: 1,
                path: new_path,
                old_path: old_path,
            });
        }
        self.refresh();
    }

    pub fn delete_selected(&mut self) {
        let tab = self.active_tab;
        let idx = match self.tabs[tab].sel_idx {
            Some(i) => i,
            None => return,
        };
        let path = self.tabs[tab].entries[idx].path.clone();
        if delete_file(&path).is_ok() {
            self.ops.push(FileOpLog {
                op_type: 0,
                path: path.clone(),
                old_path: String::new(),
            });
        }
        self.refresh();
    }

    pub fn create_folder(&mut self, name: &str) {
        let base = self.tabs[self.active_tab].path.clone();
        let full = if base.ends_with('/') {
            alloc::format!("{}{}", base, name)
        } else {
            alloc::format!("{}/{}", base, name)
        };
        if libsarga::io::mkdir(&full, 0o755).is_ok() {
            self.ops.push(FileOpLog {
                op_type: 2,
                path: full,
                old_path: String::new(),
            });
        }
        self.refresh();
    }

    pub fn create_file(&mut self, name: &str) {
        let base = self.tabs[self.active_tab].path.clone();
        let full = if base.ends_with('/') {
            alloc::format!("{}{}", base, name)
        } else {
            alloc::format!("{}/{}", base, name)
        };
        if libsarga::fs::write_file(&full, "").is_ok() {
            self.ops.push(FileOpLog {
                op_type: 2,
                path: full,
                old_path: String::new(),
            });
        }
        self.refresh();
    }

    pub fn duplicate_selected(&mut self) {
        let tab = self.active_tab;
        let idx = match self.tabs[tab].sel_idx {
            Some(i) => i,
            None => return,
        };
        let src = &self.tabs[tab].entries[idx];
        let new_name = alloc::format!("copy_{}", src.name);
        let base = self.tabs[self.active_tab].path.clone();
        let dest = if base.ends_with('/') {
            alloc::format!("{}{}", base, new_name)
        } else {
            alloc::format!("{}/{}", base, new_name)
        };
        let _ = copy_file(&src.path, &dest);
        self.refresh();
    }

    pub fn undo_last_op(&mut self) {
        if let Some(op) = self.ops.pop() {
            match op.op_type {
                0 => {
                    let _ = libsarga::posix::rename(&op.old_path, &op.path);
                }
                1 => {
                    let _ = libsarga::posix::rename(&op.path, &op.old_path);
                }
                2 => {
                    let _ = libsarga::io::unlink(&op.path);
                }
                _ => {}
            }
        }
        self.refresh();
    }

    // ---- Trash ----

    const TRASH_PATH: &'static str = "/tmp/trash";
    const TRASH_META: &'static str = "/tmp/trash/meta";

    pub fn trash_selected(&mut self) {
        let tab = self.active_tab;
        let idx = match self.tabs[tab].sel_idx {
            Some(i) => i,
            None => return,
        };
        let src = &self.tabs[tab].entries[idx].path;
        let name = &self.tabs[tab].entries[idx].name;
        let trash_dest = alloc::format!("{}/{}", Self::TRASH_PATH, name);
        // Ensure trash directory exists
        let _ = libsarga::io::mkdir(Self::TRASH_PATH, 0o755);
        // Store metadata for restore
        let _ = libsarga::io::mkdir(Self::TRASH_META, 0o755);
        let meta_path = alloc::format!("{}/{}", Self::TRASH_META, name);
        let _ = libsarga::fs::write_file(&meta_path, src);
        // Move to trash
        if rename_file(src, &trash_dest).is_ok() {
            self.ops.push(FileOpLog {
                op_type: 1,
                path: trash_dest,
                old_path: String::from(src),
            });
        }
        self.refresh();
    }

    pub fn restore_from_trash(&mut self, name: &str) {
        let meta_path = alloc::format!("{}/{}", Self::TRASH_META, name);
        if let Ok(orig_path) = libsarga::io::read_to_string(&meta_path) {
            let trash_path = alloc::format!("{}/{}", Self::TRASH_PATH, name);
            if rename_file(&trash_path, &orig_path).is_ok() {
                let _ = libsarga::io::unlink(&meta_path);
                self.ops.push(FileOpLog {
                    op_type: 1,
                    path: orig_path,
                    old_path: trash_path,
                });
            }
        }
        self.refresh();
    }

    pub fn empty_trash(&mut self) {
        let entries = crate::vfs::VfsContext::list_dir_static(Self::TRASH_PATH);
        for e in &entries {
            if e.is_dir {
                continue;
            }
            let _ = libsarga::io::unlink(&e.path);
        }
        self.refresh();
    }

    pub fn permanent_delete_selected(&mut self) {
        let tab = self.active_tab;
        let path = match self.tabs[tab]
            .sel_idx
            .map(|i| self.tabs[tab].entries[i].path.clone())
        {
            Some(p) => p,
            None => return,
        };
        let _ = libsarga::io::unlink(&path);
        self.refresh();
    }

    // ---- Search ----

    pub fn start_search(&mut self, query: &str) {
        self.search_query = String::from(query);
        self.search_results.clear();
        if query.is_empty() {
            self.search_active = false;
            return;
        }
        self.search_active = true;
        let base = self.tabs[self.active_tab].path.clone();
        recursive_search(&base, query, &mut self.search_results);
    }

    pub fn cancel_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_results.clear();
    }

    // ---- Preview ----

    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
        self.preview_content.clear();
        if self.show_preview {
            self.load_preview();
        }
    }

    pub fn load_preview(&mut self) {
        self.preview_content.clear();
        let tab = self.active_tab;
        let idx = match self.tabs[tab].sel_idx {
            Some(i) => i,
            None => return,
        };
        let path = &self.tabs[tab].entries[idx].path;
        if self.tabs[tab].entries[idx].is_dir {
            return;
        }
        if let Ok(content) = libsarga::io::read_to_string(path) {
            let preview = if content.len() > 4000 {
                &content[..4000]
            } else {
                &content
            };
            self.preview_content.push_str(preview);
        }
    }

    // ---- Favorites ----

    pub fn toggle_favorite(&mut self, path: &str) {
        if let Some(i) = self.favorites.iter().position(|p| p == path) {
            self.favorites.remove(i);
        } else {
            self.favorites.push(String::from(path));
        }
    }

    pub fn is_favorite(&self, path: &str) -> bool {
        self.favorites.iter().any(|p| p == path)
    }

    // ---- Drag & Drop ----

    pub fn drag_source(&mut self) -> Option<(String, String)> {
        let tab = self.active_tab;
        self.tabs[tab].sel_idx.map(|i| {
            (
                self.tabs[tab].entries[i].path.clone(),
                self.tabs[tab].entries[i].name.clone(),
            )
        })
    }

    pub fn handle_drop(&mut self, src_path: &str, dest_dir: &str) {
        let name = src_path.rsplit('/').next().unwrap_or(src_path);
        let dest = if dest_dir.ends_with('/') {
            alloc::format!("{}{}", dest_dir, name)
        } else {
            alloc::format!("{}/{}", dest_dir, name)
        };
        if rename_file(src_path, &dest).is_ok() {
            self.ops.push(FileOpLog {
                op_type: 1,
                path: dest.clone(),
                old_path: String::from(src_path),
            });
        }
        self.refresh();
    }

    // ---- Properties ----

    pub fn get_properties(&self) -> Vec<(String, String)> {
        let tab = self.active_tab;
        let mut props = Vec::new();
        if let Some(idx) = self.tabs[tab].sel_idx {
            let e = &self.tabs[tab].entries[idx];
            props.push((String::from("Name"), e.name.clone()));
            props.push((
                String::from("Type"),
                if e.is_dir {
                    String::from("Directory")
                } else {
                    e.name
                        .rfind('.')
                        .map(|i| String::from(&e.name[i + 1..]))
                        .unwrap_or(String::from("File"))
                },
            ));
            props.push((String::from("Size"), format_size(e.size)));
            props.push((String::from("Path"), e.path.clone()));
            props.push((String::from("Modified"), format_date(e.modified)));
            props.push((String::from("Is Dir"), alloc::format!("{}", e.is_dir)));
        }
        props
    }

    // ---- Open With ----

    pub fn get_open_with_candidates(&self, ext: &str) -> Vec<&'static str> {
        match ext {
            ".txt" | ".md" | ".rs" | ".c" | ".h" | ".py" | ".js" | ".toml" => {
                alloc::vec!["/bin/skyedit", "/bin/sargaview"]
            }
            ".png" | ".jpg" | ".bmp" | ".gif" => alloc::vec!["/bin/sargaview"],
            _ => alloc::vec!["/bin/sargaview"],
        }
    }

    pub fn open_with(&mut self, app_path: &str) {
        let tab = self.active_tab;
        let path = match self.tabs[tab]
            .sel_idx
            .map(|i| self.tabs[tab].entries[i].path.clone())
        {
            Some(p) => p,
            None => return,
        };
        match libsarga::process::fork() {
            Ok(0) => {
                let _ = libsarga::process::execve(app_path, &[app_path, &path], &[]);
                libsarga::process::exit(1);
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    // ---- Keyboard ----

    pub fn handle_key(&mut self, key: u8) {
        match key {
            0x0D => {
                // Enter — enter dir
                if let Some(idx) = self.tabs[self.active_tab].sel_idx {
                    let path = self.tabs[self.active_tab].entries[idx].path.clone();
                    self.enter_dir(&path);
                }
            }
            0x7F | 0x08 => {
                // Backspace — go up
                self.go_up();
            }
            b'1' | b'2' | b'3' | b'4' => {
                self.tabs[self.active_tab].view_mode = key - b'1';
                self.refresh();
            }
            b'p' => {
                self.toggle_preview();
            }
            b'f' => {
                let path = self.tabs[self.active_tab].path.clone();
                self.toggle_favorite(&path);
            }
            b'/' => {
                self.search_active = !self.search_active;
            }
            0x26 => {
                // Arrow Up
                let tab = &mut self.tabs[self.active_tab];
                if let Some(idx) = tab.sel_idx {
                    if idx > 0 {
                        tab.sel_idx = Some(idx - 1);
                    }
                } else if !tab.entries.is_empty() {
                    tab.sel_idx = Some(tab.entries.len() - 1);
                }
            }
            0x28 => {
                // Arrow Down
                let tab = &mut self.tabs[self.active_tab];
                if let Some(idx) = tab.sel_idx {
                    if idx + 1 < tab.entries.len() {
                        tab.sel_idx = Some(idx + 1);
                    }
                } else if !tab.entries.is_empty() {
                    tab.sel_idx = Some(0);
                }
            }
            b'z' => {
                self.undo_last_op();
            }
            _ => {}
        }
    }

    pub fn status_text(&self) -> String {
        let tab = self.active_tab;
        let count = if self.search_active {
            self.search_results.len()
        } else {
            self.tabs[tab].entries.len()
        };
        let sel = self.tabs[tab].sel_idx.map(|_| "1 selected").unwrap_or("");
        alloc::format!("{} items  {}", count, sel)
    }
}

fn recursive_search(dir: &str, query: &str, results: &mut Vec<VfsEntry>) {
    recursive_search_depth(dir, query, results, 0);
}

fn recursive_search_depth(dir: &str, query: &str, results: &mut Vec<VfsEntry>, depth: u32) {
    // ponytail: depth limit 10, reopen if needed
    if depth > 10 {
        return;
    }
    let query_lower = query.to_lowercase();
    if let Ok(fd) = libsarga::io::open(dir, 0) {
        let mut buf = [0u8; 4096];
        loop {
            let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
            if n <= 0 {
                break;
            }
            let mut off = 0usize;
            while off + 19 <= n as usize {
                let ino = u64::from_ne_bytes(buf[off..off + 8].try_into().unwrap_or([0; 8]));
                if ino == 0 {
                    break;
                }
                let _type = buf[off + 16];
                off += 17;
                let name_end = off + buf[off..].iter().position(|&b| b == 0).unwrap_or(0);
                let name = core::str::from_utf8(&buf[off..name_end]).unwrap_or("");
                if name != "." && !name.is_empty() {
                    let full = if dir.ends_with('/') {
                        alloc::format!("{}{}", dir, name)
                    } else {
                        alloc::format!("{}/{}", dir, name)
                    };
                    let name_lower = name.to_lowercase();
                    if name_lower.contains(&query_lower) {
                        results.push(VfsEntry {
                            name: String::from(name),
                            path: full.clone(),
                            is_dir: _type == 1,
                            size: 0,
                            modified: 0,
                            file_type: _type,
                        });
                    }
                    if _type == 1 && name != "." && name != ".." {
                        recursive_search_depth(&full, query, results, depth + 1);
                    }
                }
                off = name_end + 1;
            }
        }
        let _ = libsarga::io::close(fd);
    }
}

// Raw file I/O helpers
fn copy_file(src: &str, dest: &str) -> Result<(), ()> {
    let src_fd = libsarga::io::open(src, 0).map_err(|_| ())?;
    let mut buf = [0u8; 4096];
    let dest_fd = libsarga::io::open(dest, 0x42).map_err(|_| {
        let _ = libsarga::io::close(src_fd);
        ()
    })?;
    loop {
        let n = libsarga::io::read(src_fd, &mut buf).unwrap_or(0);
        if n <= 0 {
            break;
        }
        let mut written = 0;
        while written < n {
            let w = libsarga::io::write(dest_fd, &buf[written..n]).unwrap_or(0);
            if w == 0 {
                break;
            }
            written += w;
        }
    }
    let _ = libsarga::io::close(src_fd);
    let _ = libsarga::io::close(dest_fd);
    Ok(())
}

fn rename_file(old: &str, new: &str) -> Result<(), ()> {
    if libsarga::posix::rename(old, new) == 0 {
        Ok(())
    } else {
        Err(())
    }
}

fn delete_file(path: &str) -> Result<(), ()> {
    if libsarga::io::unlink(path).is_ok() {
        Ok(())
    } else {
        Err(())
    }
}

pub(crate) fn sort_entries(entries: &mut [VfsEntry], by: u8, desc: bool) {
    fn cmp_name(a: &VfsEntry, b: &VfsEntry) -> core::cmp::Ordering {
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    }
    match by {
        0 => entries.sort_by(|a, b| if desc { cmp_name(b, a) } else { cmp_name(a, b) }),
        1 => entries.sort_by(|a, b| {
            let ord = a.size.cmp(&b.size);
            if desc {
                ord.reverse()
            } else {
                ord
            }
        }),
        2 => entries.sort_by(|a, b| {
            let ord = a.modified.cmp(&b.modified);
            if desc {
                ord.reverse()
            } else {
                ord
            }
        }),
        3 => entries.sort_by(|a, b| {
            let ext_a = a.name.rfind('.').map(|i| &a.name[i..]).unwrap_or("");
            let ext_b = b.name.rfind('.').map(|i| &b.name[i..]).unwrap_or("");
            let ord = ext_a.cmp(ext_b);
            if desc {
                ord.reverse()
            } else {
                ord
            }
        }),
        _ => {}
    }
    // dirs before files
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir));
}

fn format_size(s: u64) -> alloc::string::String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if s >= GB {
        alloc::format!("{}.{} GB", s / GB, (s % GB) / (GB / 10))
    } else if s >= MB {
        alloc::format!("{}.{} MB", s / MB, (s % MB) / (MB / 10))
    } else if s >= KB {
        alloc::format!("{}.{} KB", s / KB, (s % KB) / (KB / 10))
    } else {
        alloc::format!("{} B", s)
    }
}

fn format_date(ts: u64) -> alloc::string::String {
    // Simple: days since epoch
    let days = ts / 86400;
    let y = 1970 + days / 365;
    let rem = days % 365;
    let m = 1 + rem / 30;
    let d = 1 + rem % 30;
    alloc::format!("{}-{:02}-{:02}", y, m, d)
}

fn icon_for_entry(e: &VfsEntry) -> &'static str {
    if e.is_dir {
        return "\u{25B6}";
    } // ▶
    if let Some(dot) = e.name.rfind('.') {
        let ext = &e.name[dot..];
        match ext {
            ".txt" | ".md" | ".rs" | ".c" | ".h" | ".py" | ".js" | ".toml" => "\u{1F4C4}",
            ".png" | ".jpg" | ".jpeg" | ".gif" | ".bmp" | ".svg" => "\u{1F5BC}",
            ".mp3" | ".wav" | ".flac" | ".ogg" => "\u{1F3B5}",
            ".mp4" | ".avi" | ".mkv" => "\u{1F3AC}",
            ".zip" | ".tar" | ".gz" | ".7z" | ".rar" => "\u{1F4E6}",
            ".pdf" => "\u{1F4D5}",
            _ => "\u{1F4C4}",
        }
    } else {
        "\u{1F4C4}"
    }
}

pub(crate) fn draw_explorer_content(
    win: &mut Window,
    theme: &Theme,
    aw: &crate::window::AppWindow,
    explorers: &[ExplorerState],
    exp_id: u32,
) {
    let state = match explorers.iter().find(|e| e.id == exp_id) {
        Some(s) => s,
        None => return,
    };
    let tab = state.active_ref();

    let mut y = aw.y as u32 + 29;
    let bottom = aw.y as u32 + aw.h as u32 - 4;
    let area_x = aw.x as u32 + 2;
    let area_w = aw.w - 4;

    // Toolbar with nav buttons + address bar
    win.fill_rect(area_x, y, area_w, 26, theme.bg_elevated);
    // Nav buttons
    win.draw_string(area_x + 2, y + 5, "<", theme.text, 0);
    win.draw_string(area_x + 20, y + 5, ">", theme.text, 0);
    win.draw_string(area_x + 38, y + 5, "^", theme.text, 0);
    win.draw_string(area_x + 56, y + 5, "~", theme.text, 0);
    // Address bar
    let addr_x = area_x + 80;
    let addr_w = area_w - 160;
    win.draw_rounded_rect(addr_x, y + 3, addr_w, 20, 4, theme.bg_surface);
    let display_path = if tab.path.len() > (addr_w / 8) as usize {
        &tab.path[tab.path.len().saturating_sub((addr_w / 8) as usize)..]
    } else {
        &tab.path
    };
    win.draw_string(addr_x + 4, y + 6, display_path, theme.text_secondary, 0);
    // Sort type view control text
    let view_names = ["List", "Grid", "Icon", "Detl"];
    win.draw_string(
        addr_x + addr_w + 30,
        y + 5,
        view_names[tab.view_mode as usize],
        theme.text_secondary,
        0,
    );
    y += 28;

    // Search bar (when search is active)
    if state.search_active {
        win.fill_rect(area_x, y, area_w, 20, theme.bg_surface);
        win.draw_string(
            area_x + 4,
            y + 4,
            &alloc::format!(
                "Search: {} ({} results)",
                state.search_query,
                state.search_results.len()
            ),
            theme.accent,
            0,
        );
        y += 22;
    }

    let fav_width = 100u32; // favorites sidebar
    let preview_width = if state.show_preview { area_w / 3 } else { 0 };
    let main_width = if area_w > fav_width + preview_width {
        area_w - fav_width - preview_width
    } else {
        area_w
    };

    // Favorites sidebar
    let fav_x = area_x;
    if !state.favorites.is_empty() {
        win.fill_rect(fav_x, y, fav_width, bottom - y, theme.bg_surface);
        win.draw_string(fav_x + 4, y + 2, "Favorites", theme.accent, 0);
        let mut fy = y + 20;
        for fav in &state.favorites {
            let short = if fav.len() > 12 {
                &fav[fav.len() - 12..]
            } else {
                fav
            };
            if fy < bottom {
                win.draw_string(fav_x + 6, fy, short, theme.text_secondary, 0);
                fy += 14;
            }
        }
    }
    let content_x = fav_x + fav_width;
    let content_width = main_width;

    // Column headers (list/details views)
    if tab.view_mode == 0 || tab.view_mode == 3 {
        let cols = match tab.view_mode {
            0 => &["Name", "Size", "Date", "Type"][..],
            _ => &["Name", "Size", "Date", "Type", "Modified", "Path"][..],
        };
        let col_ws = [
            content_width * 35 / 100,
            content_width * 18 / 100,
            content_width * 20 / 100,
            content_width * 27 / 100,
        ];
        win.fill_rect(content_x, y, content_width, 20, theme.bg_surface);
        let mut cx = content_x + 6;
        for (i, (col, cw)) in cols.iter().zip(col_ws.iter()).enumerate() {
            let is_sort = (tab.sort_by as usize) == i;
            win.draw_string(
                cx,
                y + 4,
                col,
                if is_sort {
                    theme.accent
                } else {
                    theme.text_secondary
                },
                0,
            );
            cx += cw;
        }
        y += 22;
    }

    // entries (search results or dir listing)
    let show = if state.search_active {
        &state.search_results[..]
    } else {
        &tab.entries[..]
    };
    let item_h = if tab.view_mode == 1 {
        64u32
    } else if tab.view_mode == 2 {
        36u32
    } else {
        18u32
    };
    let available_h = bottom.saturating_sub(y);
    if !show.is_empty() && item_h > 0 {
        let max_vis = (available_h / item_h) as usize;
        let start_idx = tab.scroll as usize;
        let end = core::cmp::min(start_idx + max_vis, show.len());
        let visible = &show[start_idx..end];

        match tab.view_mode {
            0 => draw_list(
                win,
                theme,
                aw,
                content_x,
                content_width,
                y,
                item_h,
                visible,
                start_idx,
                tab,
            ),
            1 => draw_grid(
                win,
                theme,
                aw,
                content_x,
                content_width,
                y,
                item_h,
                visible,
                start_idx,
                tab,
            ),
            2 => draw_icon(
                win,
                theme,
                aw,
                content_x,
                content_width,
                y,
                item_h,
                visible,
                start_idx,
                tab,
            ),
            3 => draw_details(
                win,
                theme,
                aw,
                content_x,
                content_width,
                y,
                item_h,
                visible,
                start_idx,
                tab,
            ),
            _ => draw_list(
                win,
                theme,
                aw,
                content_x,
                content_width,
                y,
                item_h,
                visible,
                start_idx,
                tab,
            ),
        }
    }

    // Preview panel
    if state.show_preview && !state.preview_content.is_empty() {
        let pv_x = content_x + content_width;
        let pv_w = preview_width;
        win.fill_rect(
            pv_x,
            aw.y as u32 + 57,
            pv_w,
            bottom - aw.y as u32 - 57,
            theme.bg_surface,
        );
        win.draw_string(pv_x + 4, aw.y as u32 + 60, "Preview", theme.accent, 0);
        let mut py = aw.y as u32 + 72;
        for line in state.preview_content.lines().take(20) {
            if py >= bottom {
                break;
            }
            let display = if line.len() > (pv_w as usize / 2) {
                &line[..(pv_w as usize / 2)]
            } else {
                line
            };
            win.draw_string(pv_x + 4, py, display, theme.text_secondary, 0);
            py += 14;
        }
    }

    // Status bar
    let status_str = state.status_text();
    let sb_y = bottom - 16;
    win.fill_rect(area_x, sb_y, area_w, 16, theme.bg_elevated);
    win.draw_string(area_x + 4, sb_y + 2, &status_str, theme.text_secondary, 0);
}

/// Handle mouse click on an explorer window's content area.
/// Returns true if the click was consumed.
pub(crate) fn handle_explorer_click(
    state: &mut ExplorerState,
    mx: i32,
    my: i32,
    aw: &crate::window::AppWindow,
    double_click: bool,
) -> bool {
    let area_x = aw.x as u32 + 2;
    let area_w = aw.w - 4;
    let area_x_i = area_x as i32;

    // Toolbar area: nav buttons
    let toolbar_top = aw.y + 29;
    let toolbar_bot = toolbar_top + 26;
    if my >= toolbar_top && my < toolbar_bot {
        if mx >= area_x_i && mx < area_x_i + 24 {
            state.go_back();
            return true;
        }
        if mx >= area_x_i + 24 && mx < area_x_i + 48 {
            state.go_forward();
            return true;
        }
        if mx >= area_x_i + 48 && mx < area_x_i + 72 {
            state.go_up();
            return true;
        }
        if mx >= area_x_i + 72 && mx < area_x_i + 96 {
            state.home();
            return true;
        }
        if mx >= area_x_i + 96 && mx < area_x_i + 120 {
            state.refresh();
            return true;
        }
        return false;
    }

    // Column headers (list/details mode only)
    let header_top = toolbar_bot;
    let header_bot = header_top + 20;
    let is_list_like = state.active_ref().view_mode == 0 || state.active_ref().view_mode == 3;
    if is_list_like && my >= header_top && my < header_bot {
        let col_ws = [
            area_w * 35 / 100,
            area_w * 18 / 100,
            area_w * 20 / 100,
            area_w * 27 / 100,
        ];
        let mut cx = area_x;
        for col_idx in 0..4u8 {
            cx += col_ws[col_idx as usize];
            if mx < cx as i32 {
                state.set_sort(col_idx);
                return true;
            }
        }
        return false;
    }

    // Entry list area
    let list_top = if is_list_like {
        header_bot
    } else {
        toolbar_bot
    };
    let item_h = if state.active_ref().view_mode == 1 {
        64u32
    } else if state.active_ref().view_mode == 2 {
        36u32
    } else {
        18u32
    };
    if my >= list_top && item_h > 0 {
        let rel_y = (my - list_top) as u32;
        let idx = (rel_y / item_h) as usize;
        let tab_idx = state.active_tab;
        if idx < state.tabs[tab_idx].entries.len() {
            let actual_idx = idx + state.tabs[tab_idx].scroll as usize;
            if actual_idx < state.tabs[tab_idx].entries.len() {
                if double_click {
                    let is_dir = state.tabs[tab_idx].entries[actual_idx].is_dir;
                    let path = state.tabs[tab_idx].entries[actual_idx].path.clone();
                    if is_dir {
                        state.navigate(&path);
                    }
                } else {
                    state.tabs[tab_idx].sel_idx = Some(actual_idx);
                }
                return true;
            }
        }
        // Click in empty area — deselect
        state.tabs[tab_idx].sel_idx = None;
        return true;
    }
    false
}

fn draw_list(
    win: &mut Window,
    theme: &Theme,
    _aw: &crate::window::AppWindow,
    area_x: u32,
    area_w: u32,
    y: u32,
    item_h: u32,
    visible: &[VfsEntry],
    start_idx: usize,
    tab: &ExplorerTab,
) {
    let col_ws = [
        area_w * 35 / 100,
        area_w * 18 / 100,
        area_w * 20 / 100,
        area_w * 27 / 100,
    ];
    for (i, entry) in visible.iter().enumerate() {
        let abs_idx = start_idx + i;
        let ey = y + (i as u32) * item_h;
        if Some(abs_idx) == tab.sel_idx {
            win.fill_rect(
                area_x,
                ey,
                area_w,
                item_h,
                (theme.accent & 0x00FFFFFF) | 0x40000000,
            );
        }
        win.draw_string(area_x + 4, ey + 1, icon_for_entry(entry), theme.text, 0);
        let name = if entry.name.len() > 28 {
            &entry.name[..28]
        } else {
            &entry.name
        };
        win.draw_string(
            area_x + 16,
            ey + 1,
            name,
            if entry.is_dir {
                theme.accent
            } else {
                theme.text
            },
            0,
        );
        let size_str = format_size(entry.size);
        win.draw_string(
            area_x + col_ws[0] + 4,
            ey + 1,
            &size_str,
            theme.text_secondary,
            0,
        );
        let date_str = format_date(entry.modified);
        win.draw_string(
            area_x + col_ws[0] + col_ws[1] + 4,
            ey + 1,
            &date_str,
            theme.text_secondary,
            0,
        );
    }
}

fn draw_grid(
    win: &mut Window,
    theme: &Theme,
    _aw: &crate::window::AppWindow,
    area_x: u32,
    area_w: u32,
    y: u32,
    item_h: u32,
    visible: &[VfsEntry],
    start_idx: usize,
    tab: &ExplorerTab,
) {
    let cols = (area_w / 80).max(1);
    let cell_w = area_w / cols;
    for (i, entry) in visible.iter().enumerate() {
        let abs_idx = start_idx + i;
        let col = i as u32 % cols;
        let row = i as u32 / cols;
        let gx = area_x + col * cell_w + 2;
        let gy = y + row * item_h;
        if Some(abs_idx) == tab.sel_idx {
            win.fill_rect(
                gx,
                gy,
                cell_w - 4,
                item_h - 2,
                (theme.accent & 0x00FFFFFF) | 0x40000000,
            );
        }
        win.draw_string(gx + 4, gy + 4, icon_for_entry(entry), theme.text, 0);
        let name = if entry.name.len() > 10 {
            &entry.name[..10]
        } else {
            &entry.name
        };
        win.draw_string(gx + 4, gy + 22, name, theme.text_secondary, 0);
    }
}

fn draw_icon(
    win: &mut Window,
    theme: &Theme,
    _aw: &crate::window::AppWindow,
    area_x: u32,
    area_w: u32,
    y: u32,
    item_h: u32,
    visible: &[VfsEntry],
    start_idx: usize,
    tab: &ExplorerTab,
) {
    let col_ws = [
        area_w * 35 / 100,
        area_w * 18 / 100,
        area_w * 20 / 100,
        area_w * 27 / 100,
    ];
    for (i, entry) in visible.iter().enumerate() {
        let abs_idx = start_idx + i;
        let ey = y + (i as u32) * item_h;
        if Some(abs_idx) == tab.sel_idx {
            win.fill_rect(
                area_x,
                ey,
                area_w,
                item_h,
                (theme.accent & 0x00FFFFFF) | 0x40000000,
            );
        }
        win.draw_string(area_x + 4, ey + 6, icon_for_entry(entry), theme.text, 0);
        let name = if entry.name.len() > 28 {
            &entry.name[..28]
        } else {
            &entry.name
        };
        win.draw_string(
            area_x + 28,
            ey + 6,
            name,
            if entry.is_dir {
                theme.accent
            } else {
                theme.text
            },
            0,
        );
        let size_str = format_size(entry.size);
        win.draw_string(
            area_x + col_ws[0] + 4,
            ey + 6,
            &size_str,
            theme.text_secondary,
            0,
        );
        let date_str = format_date(entry.modified);
        win.draw_string(
            area_x + col_ws[0] + col_ws[1] + 4,
            ey + 6,
            &date_str,
            theme.text_secondary,
            0,
        );
    }
}

fn draw_details(
    win: &mut Window,
    theme: &Theme,
    _aw: &crate::window::AppWindow,
    area_x: u32,
    area_w: u32,
    y: u32,
    item_h: u32,
    visible: &[VfsEntry],
    start_idx: usize,
    tab: &ExplorerTab,
) {
    let col_ws = [
        area_w * 30 / 100,
        area_w * 12 / 100,
        area_w * 14 / 100,
        area_w * 14 / 100,
        area_w * 14 / 100,
        area_w * 16 / 100,
    ];
    for (i, entry) in visible.iter().enumerate() {
        let abs_idx = start_idx + i;
        let ey = y + (i as u32) * item_h;
        if Some(abs_idx) == tab.sel_idx {
            win.fill_rect(
                area_x,
                ey,
                area_w,
                item_h,
                (theme.accent & 0x00FFFFFF) | 0x40000000,
            );
        }
        let mut cx = area_x + 4;
        win.draw_string(cx, ey + 1, icon_for_entry(entry), theme.text, 0);
        let name = if entry.name.len() > 22 {
            &entry.name[..22]
        } else {
            &entry.name
        };
        win.draw_string(cx + 14, ey + 1, name, theme.text, 0);
        cx += col_ws[0];
        let size_str = format_size(entry.size);
        win.draw_string(cx, ey + 1, &size_str, theme.text_secondary, 0);
        cx += col_ws[1];
        let date_str = format_date(entry.modified);
        win.draw_string(cx, ey + 1, &date_str, theme.text_secondary, 0);
        cx += col_ws[2];
        // type
        let ext = entry
            .name
            .rfind('.')
            .map(|i| &entry.name[i..])
            .unwrap_or("");
        win.draw_string(cx, ey + 1, ext, theme.text_secondary, 0);
        cx += col_ws[3];
        // modified time
        win.draw_string(cx, ey + 1, &date_str, theme.text_secondary, 0);
        cx += col_ws[4];
        // brief path
        if let Some(slash) = entry.path.rfind('/') {
            let parent = &entry.path[..slash];
            let short = if parent.len() > 12 {
                &parent[parent.len() - 12..]
            } else {
                parent
            };
            win.draw_string(cx, ey + 1, short, theme.text_disabled, 0);
        }
    }
}
