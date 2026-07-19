//! Minimal TOML parser for update manifests
//!
//! This is a simplified TOML parser that handles the subset needed for
//! update manifests (key-value pairs and array of tables).

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Parsed TOML document
#[derive(Debug, Clone)]
pub struct TomlDocument {
    pub values: Vec<(String, TomlValue)>,
}

/// TOML value types
#[derive(Debug, Clone)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<TomlValue>),
    Table(Vec<(String, TomlValue)>),
}

impl TomlDocument {
    /// Parse a TOML string into a document
    pub fn parse(input: &str) -> Result<Self, &'static str> {
        let mut values = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            // Skip whitespace
            if c.is_whitespace() || c == '\n' || c == '\r' {
                chars.next();
                continue;
            }

            // Skip comments
            if c == '#' {
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            }

            // Parse key-value pair
            if let Ok((key, value)) = parse_key_value(&mut chars) {
                values.push((key, value));
            } else {
                // Try to parse array of tables (for [[files]])
                if c == '[' {
                    chars.next();
                    if chars.peek() == Some(&'[') {
                        chars.next(); // Second bracket
                        let table_name = parse_identifier(&mut chars)?;
                        // Skip closing brackets
                        while chars.peek() != Some(&']') {
                            chars.next();
                        }
                        chars.next(); // First ]
                        chars.next(); // Second ]

                        // Parse table content
                        let mut table_values = Vec::new();
                        while let Some(&c) = chars.peek() {
                            if c.is_whitespace() || c == '\n' || c == '\r' {
                                chars.next();
                                continue;
                            }
                            if c == '#' {
                                while let Some(&c) = chars.peek() {
                                    if c == '\n' {
                                        break;
                                    }
                                    chars.next();
                                }
                                continue;
                            }
                            if c == '[' {
                                break; // Next table or end
                            }
                            if let Ok((k, v)) = parse_key_value(&mut chars) {
                                table_values.push((k, v));
                            } else {
                                chars.next();
                            }
                        }
                        values.push((
                            table_name,
                            TomlValue::Array(vec![TomlValue::Table(table_values)]),
                        ));
                    } else {
                        // Single table - skip for now
                        while chars.peek() != Some(&']') {
                            chars.next();
                        }
                        chars.next();
                    }
                } else {
                    chars.next();
                }
            }
        }

        Ok(TomlDocument { values })
    }

    /// Get a string value by key
    pub fn get_string(&self, key: &str) -> Option<&str> {
        for (k, v) in &self.values {
            if k == key {
                if let TomlValue::String(s) = v {
                    return Some(s);
                }
            }
        }
        None
    }

    /// Get all tables for a given key (for [[files]] arrays)
    pub fn get_tables(&self, key: &str) -> Vec<&Vec<(String, TomlValue)>> {
        let mut result = Vec::new();
        for (k, v) in &self.values {
            if k == key {
                if let TomlValue::Array(arr) = v {
                    for item in arr {
                        if let TomlValue::Table(table) = item {
                            result.push(table);
                        }
                    }
                }
            }
        }
        result
    }
}

fn parse_key_value(
    chars: &mut core::iter::Peekable<core::str::Chars>,
) -> Result<(String, TomlValue), &'static str> {
    let key = parse_identifier(chars)?;

    // Skip whitespace and equals
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '=' {
            chars.next();
            break;
        } else {
            return Err("Expected '='");
        }
    }

    // Skip whitespace after equals
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let value = parse_value(chars)?;
    Ok((key, value))
}

fn parse_identifier(
    chars: &mut core::iter::Peekable<core::str::Chars>,
) -> Result<String, &'static str> {
    let mut ident = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            ident.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if ident.is_empty() {
        Err("Empty identifier")
    } else {
        Ok(ident)
    }
}

fn parse_value(
    chars: &mut core::iter::Peekable<core::str::Chars>,
) -> Result<TomlValue, &'static str> {
    let c = match chars.peek() {
        Some(&c) => c,
        None => return Err("Unexpected end"),
    };

    match c {
        '"' => {
            chars.next(); // Skip opening quote
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c == '"' {
                    chars.next();
                    break;
                }
                if c == '\\' {
                    chars.next();
                    if let Some(&esc) = chars.peek() {
                        match esc {
                            'n' => {
                                s.push('\n');
                                chars.next();
                            }
                            't' => {
                                s.push('\t');
                                chars.next();
                            }
                            'r' => {
                                s.push('\r');
                                chars.next();
                            }
                            '\\' => {
                                s.push('\\');
                                chars.next();
                            }
                            '"' => {
                                s.push('"');
                                chars.next();
                            }
                            _ => {
                                chars.next();
                            }
                        }
                    }
                } else {
                    s.push(c);
                    chars.next();
                }
            }
            Ok(TomlValue::String(s))
        }
        '\'' => {
            chars.next(); // Skip opening quote
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c == '\'' {
                    chars.next();
                    break;
                }
                s.push(c);
                chars.next();
            }
            Ok(TomlValue::String(s))
        }
        '0'..='9' | '-' => {
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '-' || c == '.' {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if num_str.contains('.') {
                Ok(TomlValue::Float(num_str.parse().unwrap_or(0.0)))
            } else {
                Ok(TomlValue::Integer(num_str.parse().unwrap_or(0)))
            }
        }
        't' | 'f' => {
            let mut bool_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphabetic() {
                    bool_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            match bool_str.as_str() {
                "true" => Ok(TomlValue::Boolean(true)),
                "false" => Ok(TomlValue::Boolean(false)),
                _ => Err("Invalid boolean"),
            }
        }
        '[' => {
            chars.next(); // Skip opening bracket
            let mut arr = Vec::new();
            while let Some(&c) = chars.peek() {
                if c == ']' {
                    chars.next();
                    break;
                }
                if c.is_whitespace() || c == ',' {
                    chars.next();
                    continue;
                }
                if let Ok(v) = parse_value(chars) {
                    arr.push(v);
                }
            }
            Ok(TomlValue::Array(arr))
        }
        _ => Err("Unexpected character in value"),
    }
}
