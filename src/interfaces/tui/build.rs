//! Build script for odd-dashboard
//! Injects build metadata (commit, timestamp, rustc version) into the binary

use std::process::Command;

fn main() {
    // Get git commit SHA
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    println!("cargo:rustc-env=BUILD_COMMIT={}", commit);
    
    // Get build timestamp
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    
    // Get Rust version
    let rustc_version = Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    println!("cargo:rustc-env=BUILD_RUSTC_VERSION={}", rustc_version);
    
    // Read version from VERSION file
    let version = std::fs::read_to_string("VERSION")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0.0.0".to_string());
    
    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    
    // Re-run if these files change
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");
}
