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
    // The suite's twenty-seven cases (docs/architecture/70-api.md — the
    // emission's roster, the funerals, the width grammar, the
    // canonical-utterance law's ban table, the key arrow's closure, and
    // the schema-bound witness): duplicate handle; missing column; extra column;
    // type-mismatched literal; the width-mismatched `bytes<N>` and
    // `interval<E, w>` selection literals (the width is the type — the
    // token→`Value` seam judges it, never `Db::create`); `closed
    // relation` without `as`; handle literal on a non-closed field; the
    // deleted inline `enum` type diagnosing its replacement; the deleted
    // `order` statement form diagnosing its derivations (the grammar
    // lock of `docs/architecture/30-dependencies.md` § refused: order
    // marks); `interval<E, 0>` (denotes nothing) and the widthless
    // `interval<E, >` (names no width), each naming the field; and the
    // window/selection ban table, each error naming the canonical form —
    // the deleted `in lo..hi per` spelling, `{1..*}` (the containment
    // respelled), `{n..n}` (write `{n}`), `{0..0}` (write `{0}`),
    // `{0..*}` (vacuous — `cardinality_zero_star`), inverted bounds, the
    // open shorthands `{..hi}` / `{lo..}`, the empty window `<={}`
    // (names no bounds), the singleton literal set (the bare literal's
    // second spelling), and the empty literal set `{}` (selects
    // nothing — write no binding); the key arrow whose right side names
    // a foreign relation (the FD reading ratified — the arrow closes
    // over its own relation, and the teaching error is spanned at the
    // offending name); the coherence check's two failing arms — a
    // containment pairing two DISAGREEING newtypes and a labeled face
    // against a bare one (the faces of a dependency agree on their
    // newtype, or neither carries one; bare↔bare passes and is pinned
    // in schema_macro.rs) — each spanned at both offending faces
    // (docs/architecture/30-dependencies.md § the taxonomy is checked);
    // and the cross-schema `FreshField`
    // witness (the schema-bound witness law — the binding typestate
    // makes a foreign witness a type mismatch).
    assert_eq!(
        seen, 27,
        "the schema compile-fail roster has twenty-seven fixtures"
    );
}
