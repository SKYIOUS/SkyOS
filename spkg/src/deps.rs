use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub struct DepConstraint {
    pub name: String,
}

pub fn parse_dep(s: &str) -> Option<DepConstraint> {
    let s = s.trim();
    for op in &[">=", "~>", "==", ">", "<"] {
        if let Some(idx) = s.find(op) {
            let name = s[..idx].trim().to_string();
            let ver = s[idx + op.len()..].trim().to_string();
            if !name.is_empty() && !ver.is_empty() {
                return Some(DepConstraint { name });
            }
        }
    }
    // bare name, no constraint
    Some(DepConstraint {
        name: s.to_string(),
    })
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
