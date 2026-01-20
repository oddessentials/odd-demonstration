//! Intentional security bugs for AI review E2E testing.
//! This file contains vulnerabilities that should be detected by AI code review.

use std::process::Command;
use std::fs;

/// Command injection - user input passed to shell
pub fn execute_command(user_input: &str) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(user_input)  // Direct user input to shell!
        .output()
        .expect("Failed to execute");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// SQL injection via format string
pub fn unsafe_query(user_id: &str) -> String {
    format!("SELECT * FROM users WHERE id = '{}'", user_id)
}

/// Path traversal - no input validation
pub fn read_file(filename: &str) -> String {
    let path = format!("/var/data/{}", filename);
    fs::read_to_string(path).unwrap_or_default()
}

/// Hardcoded credentials
pub fn get_credentials() -> (&'static str, &'static str) {
    let api_key = "AKIAIOSFODNN7EXAMPLE";
    let secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    (api_key, secret)
}

/// Unsafe unwrap that can panic
pub fn parse_config(input: &str) -> i32 {
    input.parse().unwrap()  // Will panic on invalid input
}

/// Use after free potential (unsafe block)
pub unsafe fn dangerous_pointer_math(ptr: *mut i32, offset: isize) -> i32 {
    *ptr.offset(offset)
}
