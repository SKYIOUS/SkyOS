use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::semver::Version;

// fields kept for future constraint-checking; parse_dep writes them but no reader yet
#[allow(dead_code)]
pub struct DepConstraint {
    pub name: String,
    pub operator: String, // ">=", "~>", "==", ">", "<"
    pub version: String,
}

pub fn parse_dep(s: &str) -> Option<DepConstraint> {
    let s = s.trim();
    for op in &[">=", "~>", "==", ">", "<"] {
        if let Some(idx) = s.find(op) {
            let name = s[..idx].trim().to_string();
            let ver = s[idx + op.len()..].trim().to_string();
            if !name.is_empty() && !ver.is_empty() {
                return Some(DepConstraint {
                    name,
                    operator: op.to_string(),
                    version: ver,
                });
            }
        }
    }
    // bare name, no constraint
    Some(DepConstraint {
        name: s.to_string(),
        operator: String::new(),
        version: String::new(),
    })
}

// never called: constraint checking pending until upgrade path lands
#[allow(dead_code)]
pub fn satisfies(version: &str, constraint: &DepConstraint) -> bool {
    if constraint.operator.is_empty() {
        return true;
    }
    let v = match Version::parse(version) {
        Some(v) => v,
        None => return true,
    };
    let c = match Version::parse(&constraint.version) {
        Some(c) => c,
        None => return true,
    };
    match constraint.operator.as_str() {
        ">=" => v.compare(&c) >= 0,
        ">" => v.compare(&c) > 0,
        "==" => v.compare(&c) == 0,
        "<" => v.compare(&c) < 0,
        "~>" => {
            v.major == c.major && (v.minor > c.minor || (v.minor == c.minor && v.patch >= c.patch))
        }
        _ => true,
    }
}

pub fn resolve(
    name: &str,
    index: &[super::repo::RepoIndexEntry],
    db: &[super::db::InstalledEntry],
    visited: &mut Vec<String>,
    order: &mut Vec<String>,
) -> Result<(), &'static str> {
    if visited.contains(&name.to_string()) {
        return Ok(());
    }
    visited.push(name.to_string());
    if super::db::is_installed(db, name) {
        return Ok(());
    }
    let entry = super::repo::find_in_index(index, name).ok_or("package not found in repository")?;
    for dep_str in &entry.dependencies {
        let dep = parse_dep(dep_str).ok_or("invalid dependency")?;
        resolve(&dep.name, index, db, visited, order)?;
    }
    if !order.contains(&name.to_string()) {
        order.push(name.to_string());
    }
    Ok(())
}

pub fn resolve_all(
    packages: &[String],
    index: &[super::repo::RepoIndexEntry],
    db: &[super::db::InstalledEntry],
) -> Result<Vec<String>, &'static str> {
    let mut visited = Vec::new();
    let mut order = Vec::new();
    for pkg in packages {
        resolve(pkg, index, db, &mut visited, &mut order)?;
    }
    Ok(order)
}
