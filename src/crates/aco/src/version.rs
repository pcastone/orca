//! Version information

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const VERSION_INFO: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " - ",
    env!("CARGO_PKG_AUTHORS")
);
pub const BUILD_NUMBER: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_TIMESTAMP: &str = "unknown";
pub const GIT_COMMIT_SHORT: &str = "unknown";
