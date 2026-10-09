//! Records the compiling toolchain (`rustc -vV`) for report provenance, so the
//! nightly pin lives only in `rust-toolchain.toml`.
use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(&rustc)
        .arg("-vV")
        .output()
        .expect("run rustc -vV");
    assert!(output.status.success(), "rustc -vV failed");
    let text = String::from_utf8(output.stdout).expect("rustc -vV prints UTF-8");
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .map_or("unknown", str::trim)
    };
    let release = text.lines().next().unwrap_or("rustc unknown");
    println!(
        "cargo:rustc-env=BUMBLEDB_BENCH_RUSTC={release}; host {}; LLVM {}",
        field("host:"),
        field("LLVM version:")
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUSTC");
}
