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
//! artifacts — no second cargo build, no version skew. Nightly-2026-08-15
//! cargo (build-dir layout v2) stores each unit under
//! `target/<profile>/build/<pkg>/<hash>/out/` instead of a single `deps`
//! directory; the runner searches those `out` dirs (and still understands
//! the legacy `deps` layout if an opt-out restored it). The
//! `bumbledb_query_macros` proc-macro dylib resolves through the
//! `-L dependency=` search paths.

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

/// Cargo search directories for rlibs and proc-macro dylibs.
///
/// Nightly-2026-08-15 cargo (build-dir layout v2) stores each unit under
/// `target/<profile>/build/<pkg>/<hash>/out/` instead of a single `deps`
/// directory. The runner still understands the legacy `deps` layout.
///
/// After a toolchain bump, CI `restore-keys` can leave a previous rustc's
/// `deps/` tree next to a freshly rebuilt v2 layout. Returning only `deps/`
/// then feeds rustc an incompatible rlib (E0514) and the first fixture
/// dies without its pinned diagnostic. Search every live layout and pick
/// a current-rustc artifact below.
fn search_dirs() -> Vec<PathBuf> {
    let exe = std::env::current_exe().expect("the test binary knows its path");
    let mut dirs = Vec::new();
    if let Some(build) = exe
        .ancestors()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("build"))
    {
        dirs.extend(unit_out_dirs(build));
    }
    if let Some(deps) = legacy_deps(&exe) {
        dirs.push(deps);
    }
    assert!(
        !dirs.is_empty(),
        "no cargo artifact directories above {}",
        exe.display()
    );
    dirs
}

fn legacy_deps(exe: &Path) -> Option<PathBuf> {
    for ancestor in exe.ancestors() {
        let candidate = if ancestor.file_name().and_then(|n| n.to_str()) == Some("deps") {
            ancestor.to_path_buf()
        } else {
            ancestor.join("deps")
        };
        if candidate.is_dir() && dir_has_artifact(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn unit_out_dirs(build: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for pkg in std::fs::read_dir(build).expect("read cargo build dir") {
        let pkg = pkg.expect("pkg entry").path();
        if !pkg.is_dir() {
            continue;
        }
        let Ok(hashes) = std::fs::read_dir(&pkg) else {
            continue;
        };
        for hash in hashes {
            let out = hash.expect("hash entry").path().join("out");
            if out.is_dir() {
                dirs.push(out);
            }
        }
    }
    dirs
}

fn dir_has_artifact(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            matches!(
                entry.path().extension().and_then(|ext| ext.to_str()),
                Some("rlib" | "dylib" | "so")
            )
        })
    })
}

/// Candidates for one crate: `lib{name}-{hash}.{ext…}`, newest first.
/// Newest-by-mtime prefers the current build when feature variants left
/// siblings behind; rustc compatibility then drops a previous toolchain's
/// leftover (CI cache restore after a rustc pin move).
fn artifact_candidates(dirs: &[PathBuf], name: &str, extensions: &[&str]) -> Vec<PathBuf> {
    let prefix = format!("lib{name}-");
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries {
            let entry = entry.expect("artifact entry");
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
            found.push((modified, path));
        }
    }
    found.sort_by_key(|a| std::cmp::Reverse(a.0));
    found.into_iter().map(|(_, path)| path).collect()
}

fn rustc_accepts_rlib(
    rustc: &str,
    name: &str,
    artifact: &Path,
    search: &[PathBuf],
    scratch: &Path,
) -> bool {
    let probe = scratch.join(format!("__compat_{name}.rs"));
    std::fs::write(&probe, format!("extern crate {name};\n")).expect("write rustc compat probe");
    let mut command = Command::new(rustc);
    command
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--out-dir")
        .arg(scratch);
    for dir in search {
        command
            .arg("-L")
            .arg(format!("dependency={}", dir.display()));
    }
    let output = command
        .arg("--extern")
        .arg(format!("{name}={}", artifact.display()))
        .arg(&probe)
        .output()
        .expect("spawn rustc compat probe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    !stderr.contains("E0514") && !stderr.contains("incompatible version of rustc")
}

fn compatible_artifact(
    dirs: &[PathBuf],
    name: &str,
    extensions: &[&str],
    rustc: &str,
    scratch: &Path,
) -> PathBuf {
    let candidates = artifact_candidates(dirs, name, extensions);
    assert!(
        !candidates.is_empty(),
        "no lib{name} artifact in cargo unit out dirs"
    );
    let mut rejected = Vec::new();
    for path in &candidates {
        if rustc_accepts_rlib(rustc, name, path, dirs, scratch) {
            return path.clone();
        }
        rejected.push(path.display().to_string());
    }
    panic!(
        "no rustc-compatible lib{name} artifact (E0514 on every candidate: {})",
        rejected.join(", ")
    );
}

/// Compiles one fixture, expecting failure with the pinned diagnostics.
fn check_fixture(
    fixture: &Path,
    search: &[PathBuf],
    out_dir: &Path,
    bumbledb: &Path,
    query_facade: &Path,
    rustc: &str,
) {
    let source = std::fs::read_to_string(fixture).expect("read fixture");
    let expected = expectation(&source, fixture);
    let mut command = Command::new(rustc);
    command
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--out-dir")
        .arg(out_dir);
    for dir in search {
        command
            .arg("-L")
            .arg(format!("dependency={}", dir.display()));
    }
    let output = command
        .arg("--extern")
        .arg(format!("bumbledb={}", bumbledb.display()))
        .arg("--extern")
        .arg(format!("bumbledb_query={}", query_facade.display()))
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
    let search = search_dirs();
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let out_dir = std::env::temp_dir().join(format!(
        "bumbledb-query-compile-fail-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("create scratch out-dir");
    let bumbledb = compatible_artifact(&search, "bumbledb", &["rlib"], &rustc, &out_dir);
    let query_facade = compatible_artifact(&search, "bumbledb_query", &["rlib"], &rustc, &out_dir);
    let mut seen = 0;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&fixtures)
        .expect("read the fixture dir")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();
    entries.sort();
    for fixture in entries {
        check_fixture(
            &fixture,
            &search,
            &out_dir,
            &bumbledb,
            &query_facade,
            &rustc,
        );
        seen += 1;
    }
    let _ = std::fs::remove_dir_all(&out_dir);
    // The roster covers every deliberate spanned refusal: typo'd
    // relation, typo'd field, ambiguous punning, ?param in a head, `:-`
    // anywhere, a query with no bare main rule, an explicitly
    // indexed dense interior/rec list, mixed bare + indexed interior/rec
    // bindings, an UpperCamel derived-table name, a lowercase relation
    // respelling, an atom under a condition tree, an empty tree node, an
    // interior/rec taking a reserved tree name, a dropped body comma, param
    // mixing in both directions, a bare handle at an interior/rec position,
    // an unbound head variable, a
    // negative `u64`, a foreign integer suffix, a binding's `in` without
    // its ?param, a numeric label on a relation atom, a third Arg
    // position after the key.
    assert_eq!(seen, 32, "the compile-fail roster has thirty-two fixtures");
}
