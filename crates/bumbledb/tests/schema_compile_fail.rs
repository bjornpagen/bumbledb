//! The `schema!` compile-fail suite, hand-rolled (no `trybuild` — the
//! dependency law; `bumbledb-query`'s runner is the precedent): each
//! fixture under `tests/schema-compile-fail/` must **fail** to compile,
//! and its `//@ error: <substring>` directives (repeatable) pin the
//! diagnostic. The macro's grammar checks are expansion panics, so every
//! diagnostic is spanned at the invocation — no `//@ line` directives.
//!
//! The runner drives `rustc` directly against the workspace's own build
//! artifacts: this integration test lives in `target/…/deps`, so its
//! parent directory holds the `bumbledb` rlib (and, transitively
//! discoverable through `-L dependency=`, the `bumbledb-macros`
//! proc-macro library) — no second cargo build, no version skew.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One fixture's pinned diagnostics.
fn expected_errors(source: &str, fixture: &Path) -> Vec<String> {
    let errors: Vec<String> = source
        .lines()
        .filter_map(|text| text.trim().strip_prefix("//@ error:"))
        .map(|rest| rest.trim().to_owned())
        .collect();
    assert!(
        !errors.is_empty(),
        "fixture {} declares no //@ error directive",
        fixture.display()
    );
    errors
}

/// The deps directory of the build that produced this test binary.
fn deps_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("the test binary knows its path");
    exe.parent().expect("deps dir").to_path_buf()
}

/// The newest artifact for one crate: `lib{name}-{hash}.{ext…}` in the
/// deps dir. Newest-by-mtime picks the current build when feature
/// variants left siblings behind; any variant carries the surface the
/// fixtures use.
fn newest_artifact(deps: &Path, name: &str, extensions: &[&str]) -> PathBuf {
    let prefix = format!("lib{name}-");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(deps).expect("read deps dir") {
        let entry = entry.expect("deps entry");
        let path = entry.path();
        let Some(file) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let matches = file.starts_with(&prefix)
            && extensions
                .iter()
                .any(|ext| path.extension().and_then(|e| e.to_str()) == Some(*ext));
        if !matches {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .expect("artifact mtime");
        if best.as_ref().is_none_or(|(when, _)| modified > *when) {
            best = Some((modified, path));
        }
    }
    best.map_or_else(
        || panic!("no lib{name} artifact under {}", deps.display()),
        |(_, path)| path,
    )
}

/// Compiles one fixture, expecting failure with the pinned diagnostics.
fn check_fixture(fixture: &Path, deps: &Path, out_dir: &Path) {
    let source = std::fs::read_to_string(fixture).expect("read fixture");
    let expected = expected_errors(&source, fixture);
    let bumbledb = newest_artifact(deps, "bumbledb", &["rlib"]);
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(rustc)
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--out-dir")
        .arg(out_dir)
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--extern")
        .arg(format!("bumbledb={}", bumbledb.display()))
        .arg(fixture)
        .output()
        .expect("spawn rustc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "{} compiled — it must fail\n{stderr}",
        fixture.display()
    );
    for needle in &expected {
        assert!(
            stderr.contains(needle),
            "{} failed without the pinned diagnostic `{needle}`\n{stderr}",
            fixture.display()
        );
    }
}

#[test]
fn schema_compile_fail_fixtures() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/schema-compile-fail");
    let deps = deps_dir();
    let out_dir = std::env::temp_dir().join(format!(
        "bumbledb-schema-compile-fail-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("create scratch out-dir");
    let mut seen = 0;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&fixtures)
        .expect("read the fixture dir")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();
    entries.sort();
    for fixture in entries {
        check_fixture(&fixture, &deps, &out_dir);
        seen += 1;
    }
    let _ = std::fs::remove_dir_all(&out_dir);
    // The suite's seven cases (docs/architecture/70-api.md — the emission's roster,
    // plus the enum funeral): duplicate handle; missing column; extra
    // column; type-mismatched literal; `closed relation` without `as`;
    // handle literal on a non-closed field; the deleted inline `enum`
    // type diagnosing its replacement.
    assert_eq!(seen, 7, "the schema compile-fail roster has seven fixtures");
}
