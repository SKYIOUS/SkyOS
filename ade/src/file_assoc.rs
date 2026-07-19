//! File association engine — extension/mime → application mapping.
#![allow(dead_code)]

use alloc::vec::Vec;
use crate::app_db::APPS;

pub(crate) struct FileAssoc {
    pub ext: &'static str,
    pub mime: &'static str,
    pub app_exec: &'static str,
}

pub(crate) struct FileAssociationEngine {
    pub assocs: Vec<FileAssoc>,
}

impl FileAssociationEngine {
    pub fn new() -> Self {
        let mut assocs = Vec::new();
        assocs.push(FileAssoc { ext: "txt", mime: "text/plain", app_exec: "/bin/skyedit" });
        assocs.push(FileAssoc { ext: "md",  mime: "text/markdown", app_exec: "/bin/skyedit" });
        assocs.push(FileAssoc { ext: "rs",  mime: "text/rust", app_exec: "/bin/skyedit" });
        assocs.push(FileAssoc { ext: "c",   mime: "text/x-c", app_exec: "/bin/skyedit" });
        assocs.push(FileAssoc { ext: "h",   mime: "text/x-c-header", app_exec: "/bin/skyedit" });
        assocs.push(FileAssoc { ext: "png", mime: "image/png", app_exec: "/bin/paint" });
        assocs.push(FileAssoc { ext: "jpg", mime: "image/jpeg", app_exec: "/bin/paint" });
        assocs.push(FileAssoc { ext: "bmp", mime: "image/bmp", app_exec: "/bin/paint" });
        assocs.push(FileAssoc { ext: "sh",  mime: "text/x-shell", app_exec: "/bin/sash" });
        assocs.push(FileAssoc { ext: "calc", mime: "application/x-calc", app_exec: "/bin/calculator" });
        FileAssociationEngine { assocs }
    }

    pub fn by_extension(&self, ext: &str) -> Option<&FileAssoc> {
        self.assocs.iter().find(|a| a.ext == ext)
    }

    pub fn by_mime(&self, mime: &str) -> Option<&FileAssoc> {
        self.assocs.iter().find(|a| a.mime == mime)
    }

    pub fn open_with_app(&self, ext: &str) -> Option<&'static str> {
        self.by_extension(ext).map(|a| a.app_exec)
    }

    pub fn app_name_for(&self, ext: &str) -> Option<&'static str> {
        let exec = self.open_with_app(ext)?;
        APPS.iter().find(|a| a.exec == exec).map(|a| a.name)
    }
}
