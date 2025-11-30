//! Protocol versioning for Pulse.
//!
//! This module handles protocol version negotiation and compatibility.

use serde::{Deserialize, Serialize};

/// Current protocol version.
pub const PROTOCOL_VERSION: Version = Version { major: 1, minor: 0 };

/// Protocol version information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    /// Major version - breaking changes increment this.
    pub major: u8,
    /// Minor version - backwards-compatible changes increment this.
    pub minor: u8,
}

impl Version {
    /// Create a new version.
    #[must_use]
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// Check if this version is compatible with another version.
    ///
    /// Versions are compatible if they share the same major version.
    #[must_use]
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Default for Version {
    fn default() -> Self {
        PROTOCOL_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v1_0 = Version::new(1, 0);
        let v1_1 = Version::new(1, 1);
        let v2_0 = Version::new(2, 0);

        assert!(v1_0.is_compatible_with(&v1_1));
        assert!(v1_1.is_compatible_with(&v1_0));
        assert!(!v1_0.is_compatible_with(&v2_0));
    }

    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2);
        assert_eq!(v.to_string(), "1.2");
    }
}
