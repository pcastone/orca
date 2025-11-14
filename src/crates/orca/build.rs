use std::env;

fn main() {
    // Set build number from env or default
    let build_number = env::var("BUILD_NUMBER").unwrap_or_else(|_| "0".to_string());
    println!("cargo:rustc-env=BUILD_NUMBER={}", build_number);

    // Set git commit from env or default
    let git_commit = env::var("GIT_COMMIT").unwrap_or_else(|_| {
        std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });
    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);

    // Set build timestamp
    let build_time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_time);

    // Rebuild if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
