#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub(crate) struct PackageRepository {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

pub(crate) struct RepositoryManager {
    pub repos: Vec<PackageRepository>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        RepositoryManager { repos: Vec::new() }
    }

    pub fn add(&mut self, repo: PackageRepository) {
        if !self.repos.iter().any(|r| r.name == repo.name) {
            self.repos.push(repo);
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.repos.retain(|r| r.name != name);
    }

    pub fn all(&self) -> &[PackageRepository] {
        &self.repos
    }
}
