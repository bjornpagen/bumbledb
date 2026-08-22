//! The `schema!` compile-fail suite, hand-rolled (no `trybuild` — the
//! dependency law; `bumbledb-query`'s runner is the precedent): each
//! fixture under `tests/schema-compile-fail/` must **fail** to compile,
//! and its `//@ error: <substring>` directives (repeatable) pin the
//! diagnostic. The macro's grammar and literal-typing checks are
//! expansion panics spanned at the invocation; the shared lowering's
//! issues (names, the ban table) and the parse's teaching error (the
//! key arrow's foreign right side) are `compile_error!`s at the
//! offending token; and the schema-bound-witness fixture is an ordinary
//! type mismatch — any way, no `//@ line` directives.
//!
//! The runner drives `rustc` directly against the workspace's own build
//! artifacts — no second cargo build, no version skew. Nightly-2026-08-15
//! cargo (build-dir layout v2) stores each unit under
//! `target/<profile>/build/<pkg>/<hash>/out/` instead of a single `deps`
//! directory; the runner searches those `out` dirs (and still understands
//! the legacy `deps` layout if an opt-out restored it). Proc-macro dylibs
//! resolve through the `-L dependency=` search paths.

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
fn check_fixture(fixture: &Path, search: &[PathBuf], out_dir: &Path, bumbledb: &Path, rustc: &str) {
    let source = std::fs::read_to_string(fixture).expect("read fixture");
    let expected = expected_errors(&source, fixture);
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
    let search = search_dirs();
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let out_dir = std::env::temp_dir().join(format!(
        "bumbledb-schema-compile-fail-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("create scratch out-dir");
    let bumbledb = compatible_artifact(&search, "bumbledb", &["rlib"], &rustc, &out_dir);
    let mut seen = 0;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&fixtures)
        .expect("read the fixture dir")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();
    entries.sort();
    for fixture in entries {
        check_fixture(&fixture, &search, &out_dir, &bumbledb, &rustc);
        seen += 1;
    }
    let _ = std::fs::remove_dir_all(&out_dir);
    // The suite's thirty-nine cases (the
    // emission's roster, the funerals, the width grammar, the
    // canonical-utterance law's ban table, the key arrow's closure, and
    // the schema-bound witness): duplicate handle; missing column; extra column;
    // type-mismatched literal; the width-mismatched `bytes<N>` and
    // `interval<E, w>` selection literals (the width is the type — the
    // token→`Value` seam judges it, never `Db::create`); `closed
    // relation` without `as`; handle literal on a non-closed field; the
    // deleted inline `enum` type diagnosing its replacement; the deleted
    // `order` statement form diagnosing its derivations (the grammar
    // lock); `interval<E, 0>` (denotes nothing) and the widthless
    // `interval<E, >` (names no width), each naming the field; the
    // fresh mint on a non-u64 field (fresh is legal on u64 only —
    // judged at expansion naming the field, never deferred to the
    // u64-shaped generated impls); one newtype name spanning two
    // encodings (the dedup keys on the declared encoding — the rendered
    // Rust type is lossy exactly where the interval width is the type);
    // and the
    // capacity/selection ban table, each error naming the canonical
    // form — the deleted `in lo..hi per` spelling (the standing
    // tombstone: keyword prose stays dead), unit `{1..*}` (the
    // containment respelled — the ban is unit-only; the weighted floor
    // is the positive probe in `schema_macro.rs`), `{n..n}` (write
    // `{n}`), `{0..0}` (write `{0}`),
    // `{0..*}` (vacuous — `capacity_zero_star`), inverted literal
    // bounds, the
    // open shorthands `{..hi}` / `{lo..}`, the empty window `<={}`
    // (names no bounds), the singleton literal set (the bare literal's
    // second spelling), and the empty literal set `{}` (selects
    // nothing — write no binding); the capacity typing refusals —
    // the weight path `[a.b]` (naming the pinned-column composition
    // idiom, ruling 6), the bound path `{lo..a.b}` (the same idiom —
    // one law both slots), the signed weight (polarity), the non-u64
    // weight, `[Duration(field)]` over a scalar, the bound ident off
    // TARGET's roster (C1), the signed bound, `{..Duration(field)}`
    // over a scalar, the dependent floor (hi-slot only, C6), and the
    // unit window against a Duration bound (dimension mixing, C18); the key arrow whose right side names
    // a foreign relation (the FD reading ratified — the arrow closes
    // over its own relation, and the teaching error is spanned at the
    // offending name); the determinant field spelled twice (a
    // determinant is a field set — the teaching error is spanned at the
    // second occurrence, never rustc's E0124 on the generated key
    // struct); the coherence check's two failing arms — a
    // containment pairing two DISAGREEING newtypes and a labeled face
    // against a bare one (the faces of a dependency agree on their
    // newtype, or neither carries one; bare↔bare passes and is pinned
    // in schema_macro.rs) — each spanned at both offending faces;
    // and the cross-schema `FreshField`
    // witness (the schema-bound witness law — the binding typestate
    // makes a foreign witness a type mismatch).
    assert_eq!(
        seen, 40,
        "the schema compile-fail roster has forty fixtures"
    );
}
