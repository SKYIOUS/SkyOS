//! Semantic version comparison
//!
//! Provides version comparison functionality for update checking.

use alloc::string::String;

/// Represents a semantic version (major.minor.patch)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Parse a version string (e.g., "0.0.1")
    pub fn parse(version_str: &str) -> Option<Self> {
        let parts: alloc::vec::Vec<&str> = version_str.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = if parts.len() > 2 {
            parts[2].parse().ok()?
        } else {
            0
        };

        Some(Version {
            major,
            minor,
            patch,
        })
    }

    /// Compare this version with another
    /// Returns: -1 if self < other, 0 if equal, 1 if self > other
    pub fn compare(&self, other: &Version) -> i32 {
        if self.major != other.major {
            return if self.major > other.major { 1 } else { -1 };
        }
        if self.minor != other.minor {
            return if self.minor > other.minor { 1 } else { -1 };
        }
        if self.patch != other.patch {
            return if self.patch > other.patch { 1 } else { -1 };
        }
        0
    }

    /// Check if this version is greater than another
    pub fn is_greater_than(&self, other: &Version) -> bool {
        self.compare(other) > 0
    }

    /// Check if this version is less than another
    pub fn is_less_than(&self, other: &Version) -> bool {
        self.compare(other) < 0
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        alloc::format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_parse_version_without_patch() {
        let v = Version::parse("1.2").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_compare_versions() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.2.4").unwrap();
        assert!(v1.is_less_than(&v2));
        assert!(v2.is_greater_than(&v1));
    }

    #[test]
    fn test_compare_major() {
        let v1 = Version::parse("1.9.9").unwrap();
        let v2 = Version::parse("2.0.0").unwrap();
        assert!(v1.is_less_than(&v2));
    }

    #[test]
    fn test_equal_versions() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.2.3").unwrap();
        assert_eq!(v1.compare(&v2), 0);
    }
}
