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
    search_dirs_from(&exe)
}

fn search_dirs_from(exe: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(profile) = exe
        .ancestors()
        .find(|path| {
            matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("build" | "deps")
            )
        })
        .and_then(Path::parent)
    {
        let mut profiles = vec![profile.to_path_buf()];
        // Explicit --target puts target libraries in <root>/<triple>/<profile>
        // but proc-macros remain in <root>/<profile>, even for native builds.
        if let (Some(root), Some(name)) =
            (profile.parent().and_then(Path::parent), profile.file_name())
        {
            let host = root.join(name);
            if host.is_dir() {
                profiles.push(host);
            }
        }
        for profile in profiles {
            let build = profile.join("build");
            if build.is_dir() {
                dirs.extend(unit_out_dirs(&build));
            }
            let deps = profile.join("deps");
            if dir_has_artifact(&deps) {
                dirs.push(deps);
            }
        }
    }
    assert!(
        !dirs.is_empty(),
        "no cargo artifact directories above {}",
        exe.display()
    );
    dirs
}

#[test]
fn explicit_target_search_includes_host_proc_macros_in_both_cargo_layouts() {
    let scratch =
        std::env::temp_dir().join(format!("bumbledb-artifact-dirs-{}", std::process::id()));
    std::fs::create_dir(&scratch).expect("unique artifact layout fixture");
    for layout in ["build/example/hash/out", "deps"] {
        let target = scratch
            .join("aarch64-unknown-linux-musl/debug")
            .join(layout);
        let host = scratch.join("debug").join(layout);
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&host).unwrap();
        std::fs::write(target.join("libexample-hash.rlib"), []).unwrap();
        std::fs::write(host.join("libexample_macros-hash.so"), []).unwrap();
        let dirs = search_dirs_from(&target.join("test-binary"));
        assert!(
            dirs.contains(&target),
            "target library directory was omitted"
        );
        assert!(
            dirs.contains(&host),
            "host proc-macro directory was omitted"
        );
        let native_dirs = search_dirs_from(&host.join("test-binary"));
        assert!(
            native_dirs.contains(&host),
            "implicit native layout was omitted"
        );
    }
    std::fs::remove_dir_all(&scratch).expect("remove fixture-owned scratch");
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
            // rustc re-scans every search directory for each fixture and
            // dependency. Test executables and check-only units contain no
            // linkable library; their historical output is not a search path.
            if out.is_dir() && dir_has_artifact(&out) {
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
    // Absence of a version diagnostic is not compatibility: missing or
    // partially rebuilt artifacts and missing transitive dependencies must
    // also refuse. Only a successfully compiled positive probe is evidence.
    output.status.success()
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
        "no rustc-compatible lib{name} artifact (positive probe failed for: {})",
        rejected.join(", ")
    );
}

#[test]
fn artifact_probe_requires_success_and_rejects_missing_or_malformed_libraries() {
    let scratch =
        std::env::temp_dir().join(format!("bumbledb-artifact-probe-{}", std::process::id()));
    std::fs::create_dir(&scratch).expect("unique probe directory");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let artifact = scratch.join("libbumbledb_probe.rlib");
    assert!(!rustc_accepts_rlib(
        &rustc,
        "bumbledb_probe",
        &artifact,
        &[],
        &scratch,
    ));
    std::fs::write(&artifact, b"not a Rust library").unwrap();
    assert!(!rustc_accepts_rlib(
        &rustc,
        "bumbledb_probe",
        &artifact,
        &[],
        &scratch,
    ));
    let source = scratch.join("library.rs");
    std::fs::write(&source, "pub struct PositiveProbe;\n").unwrap();
    let build = Command::new(&rustc)
        .args([
            "--edition=2021",
            "--crate-type=rlib",
            "--crate-name=bumbledb_probe",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&artifact)
        .output()
        .expect("build positive library probe");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(rustc_accepts_rlib(
        &rustc,
        "bumbledb_probe",
        &artifact,
        &[],
        &scratch,
    ));
    std::fs::remove_dir_all(&scratch).expect("remove probe-owned scratch");
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
    assert!(
        seen > 0,
        "the compile-fail suite must not silently select no cases"
    );
}
