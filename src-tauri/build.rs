use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn checked(command: &mut Command, operation: &str) -> Output {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{operation}: {error}"));
    assert!(
        output.status.success(),
        "{operation} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn build_macos_helper(manifest: &Path, out: &Path, target: &str) {
    let native = manifest.join("native/macos-helper");
    let helper_target = out.join("lid-helper-target");
    // An isolated Cargo workspace supplies official libc bindings, without Tauri or a parent lock.
    checked(
        Command::new(std::env::var_os("CARGO").expect("CARGO is missing"))
            .arg("build")
            .arg("--locked")
            .arg("--offline")
            .arg("--release")
            .args(["--jobs", "1"])
            .arg("--manifest-path")
            .arg(native.join("Cargo.toml"))
            .arg("--target")
            .arg(target)
            .arg("--target-dir")
            .arg(&helper_target)
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_MAKEFLAGS")
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS"),
        "Compile the standalone macOS lid helper",
    );
    let compiled = helper_target
        .join(target)
        .join("release/opencode-lid-helper");
    let helper = out.join("opencode-lid-helper");
    let metadata = std::fs::metadata(&compiled).expect("Compiled helper is missing");
    assert!(
        metadata.is_file() && metadata.len() > 0,
        "Invalid helper artifact"
    );
    std::fs::copy(&compiled, &helper).expect("Could not copy helper artifact");
    checked(
        Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&helper),
        "Ad-hoc sign the macOS lid helper",
    );
    checked(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--strict"])
            .arg(&helper),
        "Verify the macOS lid helper signature",
    );
    let digest = checked(
        Command::new("/usr/bin/shasum")
            .args(["-a", "256", "--"])
            .arg(&helper)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C")
            .env("LC_ALL", "C"),
        "Hash the signed macOS lid helper",
    );
    let text = std::str::from_utf8(&digest.stdout).expect("Invalid helper digest output");
    let hash = text
        .split_whitespace()
        .next()
        .expect("Missing helper digest");
    assert!(
        hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Invalid helper SHA-256"
    );
    assert!(
        std::fs::metadata(&helper)
            .expect("Signed helper is missing")
            .len()
            > 0,
        "Signed helper is empty"
    );
    println!("cargo:rustc-env=OPENCODE_LID_HELPER_SHA256={hash}");
}

fn main() {
    println!("cargo:rerun-if-changed=native/macos_bootstrap.pl");
    println!("cargo:rerun-if-changed=native/macos-helper/Cargo.toml");
    println!("cargo:rerun-if-changed=native/macos-helper/Cargo.lock");
    println!("cargo:rerun-if-changed=native/macos-helper/src");
    let target = std::env::var("TARGET").expect("TARGET is missing");
    if target.ends_with("-apple-darwin") {
        build_macos_helper(
            &PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("Missing manifest path")),
            &PathBuf::from(std::env::var_os("OUT_DIR").expect("Missing build output path")),
            &target,
        );
    }
    tauri_build::build()
}
