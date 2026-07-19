//! Configuration system — namespaced key-value settings, defaults, observers.
#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;

pub(crate) struct ConfigValue {
    pub value: String,
    pub default: String,
}

pub(crate) struct ConfigNamespace {
    pub name: &'static str,
    pub keys: Vec<(&'static str, ConfigValue)>,
}

pub(crate) struct Config {
    pub namespaces: Vec<ConfigNamespace>,
    observers: Vec<(&'static str, &'static str, fn(&str, &str))>,
}

impl Config {
    pub fn new() -> Self {
        let mut namespaces = Vec::new();
        namespaces.push(ConfigNamespace {
            name: "desktop",
            keys: vec![
                ("theme", ConfigValue { value: String::from("dark"), default: String::from("dark") }),
                ("wallpaper", ConfigValue { value: String::from(""), default: String::from("") }),
                ("sound_enabled", ConfigValue { value: String::from("true"), default: String::from("true") }),
            ],
        });
        namespaces.push(ConfigNamespace {
            name: "session",
            keys: vec![
                ("auto_restore", ConfigValue { value: String::from("true"), default: String::from("true") }),
                ("save_on_exit", ConfigValue { value: String::from("true"), default: String::from("true") }),
            ],
        });
        Config { namespaces, observers: Vec::new() }
    }

    pub fn get(&self, ns: &str, key: &str) -> Option<&str> {
        for n in &self.namespaces {
            if n.name == ns {
                for (k, v) in &n.keys {
                    if *k == key { return Some(&v.value); }
                }
            }
        }
        None
    }

    pub fn set(&mut self, ns: &str, key: &str, value: &str) {
        for n in &mut self.namespaces {
            if n.name == ns {
                for (k, v) in &mut n.keys {
                    if *k == key {
                        v.value = String::from(value);
                        for &(obs_ns, obs_key, cb) in &self.observers {
                            if obs_ns == ns && obs_key == key {
                                cb(ns, key);
                            }
                        }
                        return;
                    }
                }
            }
        }
    }

    pub fn observe(&mut self, ns: &'static str, key: &'static str, cb: fn(&str, &str)) {
        self.observers.push((ns, key, cb));
    }

    pub fn reset(&mut self, ns: &str, key: &str) {
        for n in &mut self.namespaces {
            if n.name == ns {
                for (k, v) in &mut n.keys {
                    if *k == key {
                        v.value = v.default.clone();
                        return;
                    }
                }
            }
        }
    }
}
