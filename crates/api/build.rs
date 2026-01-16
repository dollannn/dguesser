//! Build script to capture build-time metadata

use std::process::Command;

fn main() {
    // Get git SHA (short)
    let git_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get Rust version
    let rust_version = Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let version_str = String::from_utf8_lossy(&o.stdout);
                // Extract just the version number from "rustc X.Y.Z (hash date)"
                version_str.split_whitespace().nth(1).map(|s| s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get build timestamp in ISO 8601 format
    let build_timestamp = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Set environment variables for compile time
    println!("cargo:rustc-env=GIT_SHA={git_sha}");
    println!("cargo:rustc-env=RUST_VERSION={rust_version}");
    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_timestamp}");

    // Rebuild when git HEAD changes or build.rs changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=build.rs");
}
