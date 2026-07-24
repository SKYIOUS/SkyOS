use alloc::string::String;
use alloc::vec::Vec;
use libsarga::io;
use libsarga::toml::{TomlDocument, TomlValue};

const DB_PATH: &str = "/var/spkg/installed.toml";

#[derive(Clone)]
pub struct InstalledEntry {
    pub name: String,
    pub version: String,
    pub files: Vec<String>,
    pub dependencies: Vec<String>,
}

pub fn load_db() -> Vec<InstalledEntry> {
    let data = match io::read_to_string(DB_PATH) { Ok(s) => s, Err(_) => return Vec::new() };
    let doc = match TomlDocument::parse(&data) { Ok(d) => d, Err(_) => return Vec::new() };
    let tables = doc.get_tables("installed");
    let mut entries = Vec::new();
    for table in tables {
        let mut e = InstalledEntry { name: String::new(), version: String::new(), files: Vec::new(), dependencies: Vec::new() };
        for (k, v) in &*table {
            match k.as_str() {
                "name" => { if let TomlValue::String(s) = v { e.name = s.clone(); } }
                "version" => { if let TomlValue::String(s) = v { e.version = s.clone(); } }
                "files" => { if let TomlValue::Array(arr) = v { for item in arr { if let TomlValue::String(s) = item { e.files.push(s.clone()); } } } }
                "dependencies" => { if let TomlValue::Array(arr) = v { for item in arr { if let TomlValue::String(s) = item { e.dependencies.push(s.clone()); } } } }
                _ => {}
            }
        }
        if !e.name.is_empty() { entries.push(e); }
    }
    entries
}

pub fn save_db(entries: &[InstalledEntry]) -> Result<(), &'static str> {
    let mut data = String::new();
    for e in entries {
        data.push_str(&alloc::format!("[[installed]]\nname = \"{}\"\nversion = \"{}\"\nfiles = [", e.name, e.version));
        for (i, f) in e.files.iter().enumerate() {
            if i > 0 { data.push_str(", "); }
            data.push_str(&alloc::format!("\"{}\"", f));
        }
        data.push_str("]\ndependencies = [");
        for (i, d) in e.dependencies.iter().enumerate() {
            if i > 0 { data.push_str(", "); }
            data.push_str(&alloc::format!("\"{}\"", d));
        }
        data.push_str("]\n\n");
    }
    let fd = io::open(DB_PATH, 0x42).map_err(|_| "cannot open database")?; // O_RDWR | O_CREAT
    let _ = io::write(fd, data.as_bytes());
    io::close(fd).ok();
    Ok(())
}

pub fn is_installed(db: &[InstalledEntry], name: &str) -> bool {
    db.iter().any(|e| e.name == name)
}

pub fn get_installed<'a>(db: &'a [InstalledEntry], name: &str) -> Option<&'a InstalledEntry> {
    db.iter().find(|e| e.name == name)
}

pub fn get_installed_mut<'a>(db: &'a mut [InstalledEntry], name: &str) -> Option<&'a mut InstalledEntry> {
    db.iter_mut().find(|e| e.name == name)
}