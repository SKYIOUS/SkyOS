use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::net::HttpClient;
use libsarga::toml::{TomlDocument, TomlValue};

pub struct RepoConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

pub struct RepoIndexEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub filename: String,
    pub hash: String,
}

pub fn load_repos() -> Vec<RepoConfig> {
    let mut repos = Vec::new();
    let data = match libsarga::io::read_to_string("/etc/spkg/repos.conf") {
        Ok(s) => s,
        Err(_) => return repos,
    };
    let mut current = RepoConfig {
        name: String::new(),
        url: String::new(),
        enabled: false,
    };
    let mut in_section = false;
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if in_section && !current.name.is_empty() {
                repos.push(current);
                current = RepoConfig {
                    name: String::new(),
                    url: String::new(),
                    enabled: false,
                };
            }
            let section = &line[1..line.len() - 1];
            current.name = section.rsplit('.').next().unwrap_or(section).to_string();
            in_section = true;
        } else if in_section {
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let val = line[eq + 1..].trim().trim_matches('"');
                match key {
                    "url" => current.url = val.to_string(),
                    "enabled" => current.enabled = val == "true",
                    _ => {}
                }
            }
        }
    }
    if in_section && !current.name.is_empty() {
        repos.push(current);
    }
    repos
}

pub fn fetch_and_cache_index(repo: &RepoConfig) -> Result<Vec<RepoIndexEntry>, &'static str> {
    let index_url = alloc::format!("{}Packages.toml", repo.url);
    let data = HttpClient::get(&index_url).map_err(|_| "failed to fetch repository index")?;
    let cache_dir = "/var/spkg/cache/";
    let cache_path = alloc::format!("/var/spkg/cache/{}.toml", repo.name);
    let _ = libsarga::io::mkdir(cache_dir, 0o755);
    let _write = || -> Result<(), ()> {
        let fd = libsarga::io::open(&cache_path, 0x42).map_err(|_| ())?;
        let _ = libsarga::io::write(fd, &data);
        let _ = libsarga::io::close(fd);
        Ok(())
    };
    let _ = _write();
    let data_str = core::str::from_utf8(&data).map_err(|_| "invalid UTF-8 in index")?;
    parse_index(data_str)
}

pub fn load_cached_index(repo_name: &str) -> Result<Vec<RepoIndexEntry>, &'static str> {
    let cache_path = alloc::format!("/var/spkg/cache/{}.toml", repo_name);
    let data =
        libsarga::io::read_to_string(&cache_path).map_err(|_| "no cache, run spkg update first")?;
    parse_index(&data)
}

pub fn parse_index(data: &str) -> Result<Vec<RepoIndexEntry>, &'static str> {
    let doc = TomlDocument::parse(data).map_err(|_| "failed to parse index")?;
    let tables = doc.get_tables("package");
    let mut entries = Vec::new();
    for table in tables {
        let mut e = RepoIndexEntry {
            name: String::new(),
            version: String::new(),
            description: String::new(),
            dependencies: Vec::new(),
            filename: String::new(),
            hash: String::new(),
        };
        for (k, v) in table {
            match k.as_str() {
                "name" => {
                    if let TomlValue::String(s) = v {
                        e.name = s.clone();
                    }
                }
                "version" => {
                    if let TomlValue::String(s) = v {
                        e.version = s.clone();
                    }
                }
                "description" => {
                    if let TomlValue::String(s) = v {
                        e.description = s.clone();
                    }
                }
                "dependencies" => {
                    if let TomlValue::Array(arr) = v {
                        for item in arr {
                            if let TomlValue::String(s) = item {
                                e.dependencies.push(s.clone());
                            }
                        }
                    }
                }
                "filename" => {
                    if let TomlValue::String(s) = v {
                        e.filename = s.clone();
                    }
                }
                "hash" => {
                    if let TomlValue::String(s) = v {
                        e.hash = s.clone();
                    }
                }
                _ => {}
            }
        }
        if !e.name.is_empty() && !e.version.is_empty() {
            entries.push(e);
        }
    }
    Ok(entries)
}

pub fn find_in_index<'a>(entries: &'a [RepoIndexEntry], name: &str) -> Option<&'a RepoIndexEntry> {
    entries.iter().find(|e| e.name == name)
}
