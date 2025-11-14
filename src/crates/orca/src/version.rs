//! Version information for Orca
//!
//! This module provides version metadata including build number,
//! git commit, and build timestamp injected at compile time.

/// Package version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build number (from CI or default to 0)
pub const BUILD_NUMBER: &str = env!("BUILD_NUMBER");

/// Git commit hash (short form)
pub const GIT_COMMIT: &str = env!("GIT_COMMIT");

/// Build timestamp (RFC3339 format)
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");

/// Get full version information string
///
/// Returns a formatted version string including all metadata.
///
/// # Example
///
/// ```
/// use orca::version::full_version;
///
/// println!("Version: {}", full_version());
/// // Output: "Orca v0.1.0 (build 42, commit abc123, built 2025-01-15T10:30:00Z)"
/// ```
pub fn full_version() -> String {
    format!(
        "Orca v{} (build {}, commit {}, built {})",
        VERSION, BUILD_NUMBER, GIT_COMMIT, BUILD_TIMESTAMP
    )
}

/// Get short version string (version only)
///
/// # Example
///
/// ```
/// use orca::version::short_version;
///
/// println!("Version: {}", short_version());
/// // Output: "v0.1.0"
/// ```
pub fn short_version() -> String {
    format!("v{}", VERSION)
}

/// Version metadata as a struct
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: &'static str,
    pub build_number: &'static str,
    pub git_commit: &'static str,
    pub build_timestamp: &'static str,
}

impl VersionInfo {
    /// Get version metadata
    pub fn get() -> Self {
        Self {
            version: VERSION,
            build_number: BUILD_NUMBER,
            git_commit: GIT_COMMIT,
            build_timestamp: BUILD_TIMESTAMP,
        }
    }
}

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Orca v{} (build {}, commit {}, built {})",
            self.version, self.build_number, self.git_commit, self.build_timestamp
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constants() {
        assert!(!VERSION.is_empty());
        assert!(!BUILD_NUMBER.is_empty());
        assert!(!GIT_COMMIT.is_empty());
        assert!(!BUILD_TIMESTAMP.is_empty());
    }

    #[test]
    fn test_full_version() {
        let version = full_version();
        assert!(version.contains("Orca"));
        assert!(version.contains(VERSION));
    }

    #[test]
    fn test_short_version() {
        let version = short_version();
        assert!(version.starts_with("v"));
        assert!(version.contains(VERSION));
    }

    #[test]
    fn test_version_info() {
        let info = VersionInfo::get();
        assert_eq!(info.version, VERSION);
        assert_eq!(info.build_number, BUILD_NUMBER);
        assert_eq!(info.git_commit, GIT_COMMIT);
        assert_eq!(info.build_timestamp, BUILD_TIMESTAMP);
    }

    #[test]
    fn test_version_info_display() {
        let info = VersionInfo::get();
        let display = format!("{}", info);
        assert!(display.contains("Orca"));
        assert!(display.contains(VERSION));
    }
}
