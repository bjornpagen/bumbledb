//! The compile-fail suite, hand-rolled (no `trybuild` — the quarantine
//! carries zero foreign dependencies, the bench crate's own discipline):
//! each fixture under `tests/compile-fail/` must **fail** to compile, and
//! its `//@` directives pin the diagnostic —
//!
//! - `//@ error: <substring>` — the compiler output must contain it
//!   (repeatable);
//! - `//@ line: <n>` — the output must report the error at that fixture
//!   line (the punning law's "spanned at the second occurrence").
//!
//! The runner drives `rustc` directly against the workspace's own build
//! artifacts: this integration test lives in `target/…/deps`, so its
//! parent directory holds the `bumbledb` rlib and the `bumbledb_query`
//! proc-macro library the fixtures need — no second cargo build, no
//! version skew.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One fixture's parsed directives.
struct Expectation {
    errors: Vec<String>,
    line: Option<u32>,
}

fn expectation(source: &str, fixture: &Path) -> Expectation {
    let mut errors = Vec::new();
    let mut line = None;
    for text in source.lines() {
        if let Some(rest) = text.trim().strip_prefix("//@ error:") {
            errors.push(rest.trim().to_owned());
        } else if let Some(rest) = text.trim().strip_prefix("//@ line:") {
            line = Some(
                rest.trim()
                    .parse::<u32>()
                    .unwrap_or_else(|_| panic!("bad //@ line directive in {}", fixture.display())),
            );
        }
    }
    assert!(
        !errors.is_empty(),
        "fixture {} declares no //@ error directive",
        fixture.display()
    );
    Expectation { errors, line }
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
    let expected = expectation(&source, fixture);
    let bumbledb = newest_artifact(deps, "bumbledb", &["rlib"]);
    let query_macro = newest_artifact(deps, "bumbledb_query", &["dylib", "so"]);
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
        .arg("--extern")
        .arg(format!("bumbledb_query={}", query_macro.display()))
        .arg(fixture)
        .output()
        .expect("spawn rustc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "{} compiled — it must fail\n{stderr}",
        fixture.display()
    );
    for needle in &expected.errors {
        assert!(
            stderr.contains(needle),
            "{} failed without the pinned diagnostic `{needle}`\n{stderr}",
            fixture.display()
        );
    }
    if let Some(line) = expected.line {
        let file = fixture
            .file_name()
            .and_then(|n| n.to_str())
            .expect("fixture name");
        let at = format!("{file}:{line}:");
        assert!(
            stderr.contains(&at),
            "{} reported its error away from the pinned span `{at}`\n{stderr}",
            fixture.display()
        );
    }
}

#[test]
fn compile_fail_fixtures() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compile-fail");
    let deps = deps_dir();
    let out_dir = std::env::temp_dir().join(format!(
        "bumbledb-query-compile-fail-{}",
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
    // The suite's five cases (the PRD's roster): typo'd relation, typo'd
    // field, ambiguous punning, ?param in a head, `:-` anywhere.
    assert_eq!(seen, 5, "the compile-fail roster has five fixtures");
}
