//! Logging utilities
//!
//! Provides helper functions and macros for structured logging with tracing.

use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Log execution time of a function
///
/// # Example
///
/// ```rust,ignore
/// use tooling::logging::timed;
///
/// async fn slow_operation() {
///     tokio::time::sleep(Duration::from_millis(100)).await;
/// }
///
/// timed("slow_operation", slow_operation()).await;
/// ```
pub async fn timed<F, T>(name: &str, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();
    debug!("Starting: {}", name);

    let result = future.await;

    let elapsed = start.elapsed();
    debug!("Completed: {} in {:?}", name, elapsed);

    result
}

/// Log execution time with custom log level
pub async fn timed_with_level<F, T>(name: &str, level: LogLevel, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();

    match level {
        LogLevel::Debug => debug!("Starting: {}", name),
        LogLevel::Info => info!("Starting: {}", name),
        LogLevel::Warn => warn!("Starting: {}", name),
        LogLevel::Error => error!("Starting: {}", name),
    }

    let result = future.await;
    let elapsed = start.elapsed();

    match level {
        LogLevel::Debug => debug!("Completed: {} in {:?}", name, elapsed),
        LogLevel::Info => info!("Completed: {} in {:?}", name, elapsed),
        LogLevel::Warn => warn!("Completed: {} in {:?}", name, elapsed),
        LogLevel::Error => error!("Completed: {} in {:?}", name, elapsed),
    }

    result
}

/// Log levels for custom logging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// RAII guard for logging function entry and exit
///
/// Automatically logs when entering and exiting a scope.
///
/// # Example
///
/// ```rust
/// use tooling::logging::LogGuard;
///
/// fn process_data() {
///     let _guard = LogGuard::new("process_data");
///     // Function logic here
///     // Guard will log exit when dropped
/// }
/// ```
pub struct LogGuard {
    name: String,
    start: Instant,
}

impl LogGuard {
    /// Create a new log guard
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug!("Entering: {}", name);

        Self {
            name,
            start: Instant::now(),
        }
    }

    /// Create a log guard with custom level
    pub fn with_level(name: impl Into<String>, level: LogLevel) -> Self {
        let name = name.into();

        match level {
            LogLevel::Debug => debug!("Entering: {}", name),
            LogLevel::Info => info!("Entering: {}", name),
            LogLevel::Warn => warn!("Entering: {}", name),
            LogLevel::Error => error!("Entering: {}", name),
        }

        Self {
            name,
            start: Instant::now(),
        }
    }

    /// Get elapsed time since guard creation
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

impl Drop for LogGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        debug!("Exiting: {} (elapsed: {:?})", self.name, elapsed);
    }
}

/// Format duration in human-readable form
///
/// # Example
///
/// ```rust
/// use tooling::logging::format_duration;
/// use std::time::Duration;
///
/// assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
/// assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
/// assert_eq!(format_duration(Duration::from_micros(500)), "500μs");
/// ```
pub fn format_duration(duration: std::time::Duration) -> String {
    let micros = duration.as_micros();

    if micros < 1000 {
        format!("{}μs", micros)
    } else if micros < 1_000_000 {
        format!("{}ms", micros / 1000)
    } else if micros < 60_000_000 {
        format!("{:.2}s", micros as f64 / 1_000_000.0)
    } else {
        let seconds = micros / 1_000_000;
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m{}s", minutes, secs)
    }
}

/// Format bytes in human-readable form
///
/// # Example
///
/// ```rust
/// use tooling::logging::format_bytes;
///
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
/// assert_eq!(format_bytes(500), "500 B");
/// ```
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Sanitize string for logging (remove sensitive data)
///
/// Replaces common sensitive patterns with redacted markers.
///
/// # Example
///
/// ```rust
/// use tooling::logging::sanitize_for_logging;
///
/// let log = "API key: sk-abc123";
/// let sanitized = sanitize_for_logging(&log);
/// assert!(sanitized.contains("[REDACTED]"));
/// ```
pub fn sanitize_for_logging(input: &str) -> String {
    let mut result = input.to_string();

    // Redact common secret patterns
    let patterns = [
        (r"(?i)(api[\s_-]?key|apikey)\s*[:=]\s*\S+", "$1: [REDACTED]"),
        (r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+", "$1: [REDACTED]"),
        (r"(?i)(token)\s*[:=]\s*\S+", "$1: [REDACTED]"),
        (r"(?i)(secret)\s*[:=]\s*\S+", "$1: [REDACTED]"),
        (
            r"(?i)(authorization|auth)\s*:\s*bearer\s+\S+",
            "$1: Bearer [REDACTED]",
        ),
    ];

    for (pattern, replacement) in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, *replacement).to_string();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_duration_micros() {
        assert_eq!(format_duration(Duration::from_micros(500)), "500μs");
    }

    #[test]
    fn test_format_duration_millis() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(Duration::from_secs(125)), "2m5s");
    }

    #[test]
    fn test_format_bytes_small() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(2048), "2.00 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_sanitize_api_key() {
        let input = "API key: sk-abc123";
        let sanitized = sanitize_for_logging(input);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("sk-abc123"));
    }

    #[test]
    fn test_sanitize_password() {
        let input = "password: secret123";
        let sanitized = sanitize_for_logging(input);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("secret123"));
    }

    #[test]
    fn test_sanitize_token() {
        let input = "token=xyz789";
        let sanitized = sanitize_for_logging(input);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("xyz789"));
    }

    #[test]
    fn test_sanitize_bearer_token() {
        let input = "Authorization: Bearer abc123xyz";
        let sanitized = sanitize_for_logging(input);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("abc123xyz"));
    }

    #[test]
    fn test_sanitize_preserves_safe_data() {
        let input = "User: john@example.com, Status: active";
        let sanitized = sanitize_for_logging(input);
        assert_eq!(input, sanitized);
    }

    #[test]
    fn test_log_guard() {
        let _guard = LogGuard::new("test_function");
        // Guard should log on drop
    }

    #[test]
    fn test_log_guard_elapsed() {
        let guard = LogGuard::new("test");
        std::thread::sleep(Duration::from_millis(10));
        assert!(guard.elapsed() >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_timed() {
        let result = timed("test_operation", async { 42 }).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_timed_with_level() {
        let result = timed_with_level("test", LogLevel::Info, async { "success" }).await;
        assert_eq!(result, "success");
    }
}
