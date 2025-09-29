
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Get git commit hash
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Get or increment build number
    let build_number = get_or_increment_build_number();
    
    // Set environment variables for use in the binary
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_NUMBER={}", build_number);
    
    // Tell cargo to rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/main");
    println!("cargo:rerun-if-changed=build_number.txt");
}

fn get_or_increment_build_number() -> u32 {
    let build_file = "build_number.txt";
    
    // Read existing build number or start at 1
    let current_build = if Path::new(build_file).exists() {
        fs::read_to_string(build_file)
            .unwrap_or_else(|_| "0".to_string())
            .trim()
            .parse::<u32>()
            .unwrap_or(0)
    } else {
        0
    };
    
    let new_build = current_build + 1;
    
    // Write new build number
    if let Err(e) = fs::write(build_file, new_build.to_string()) {
        eprintln!("Warning: Could not write build number: {}", e);
    }
    
    new_build
}
