//! Every `.rs` file under `tests/compile_fail/` must fail to compile, with
//! diagnostics containing each `//@ error: <text>` and, if given, pointing at
//! `//@ line: <n>`. `bumbledb` is built once into a private target directory;
//! cargo's JSON messages name every artifact, so the runner never reads cargo's
//! directory layout. Each fixture is one `rustc` run.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

struct Library {
    rmeta: PathBuf,
    search: BTreeSet<PathBuf>,
}

/// The JSON string value after `"key":` in one cargo message line.
fn string_after<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let start = line.find(&format!("\"{key}\":\""))? + key.len() + 4;
    let end = start + line[start..].find('"')?;
    Some(&line[start..end])
}

/// The `"filenames":[…]` paths of one `compiler-artifact` message.
fn filenames(line: &str) -> Vec<PathBuf> {
    let Some(start) = line.find("\"filenames\":[") else {
        return Vec::new();
    };
    let list = &line[start + 13..];
    let list = &list[..list.find(']').unwrap_or(list.len())];
    list.split(',')
        .map(|item| PathBuf::from(item.trim().trim_matches('"').replace("\\\\", "\\")))
        .filter(|path| !path.as_os_str().is_empty())
        .collect()
}

fn library(scratch: &Path) -> Library {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO"))
        .current_dir(manifest_dir)
        .args(["build", "--lib", "--offline", "--message-format=json"])
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(scratch.join("target"))
        .output()
        .expect("run cargo build");
    assert!(
        output.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut rmeta = None;
    let mut search = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if string_after(line, "reason") != Some("compiler-artifact") {
            continue;
        }
        let target = &line[line.find("\"target\":").unwrap_or(0)..];
        let files = filenames(line);
        if string_after(target, "name") == Some("bumbledb") {
            rmeta = files
                .iter()
                .find(|file| file.extension().is_some_and(|ext| ext == "rmeta"))
                .cloned();
        }
        search.extend(
            files
                .iter()
                .filter_map(|file| file.parent().map(Path::to_path_buf)),
        );
    }
    Library {
        rmeta: rmeta.expect("cargo reported bumbledb's metadata"),
        search,
    }
}

fn fixtures(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read the fixture directory") {
        let path = entry.expect("a fixture entry").path();
        if path.is_dir() {
            fixtures(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Compiles one fixture; `Err` describes how it missed its directives.
fn check(fixture: &Path, library: &Library, scratch: &Path) -> Result<(), String> {
    let source = std::fs::read_to_string(fixture).expect("read the fixture");
    let directive = |name: &str| {
        source
            .lines()
            .filter_map(|line| line.trim().strip_prefix(name).map(str::trim))
            .collect::<Vec<_>>()
    };
    let errors = directive("//@ error:");
    if errors.is_empty() {
        return Err("declares no `//@ error:` directive".to_owned());
    }
    let stem = fixture
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("a UTF-8 file name");
    let group = fixture
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default();
    let out = scratch.join("out").join(group).join(stem);
    std::fs::create_dir_all(&out).expect("create the fixture output directory");
    let mut rustc = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
    rustc
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg("--crate-name")
        .arg(stem)
        .arg("--out-dir")
        .arg(&out)
        .arg("--extern")
        .arg(format!("bumbledb={}", library.rmeta.display()));
    for dir in &library.search {
        rustc.arg("-L").arg(format!("dependency={}", dir.display()));
    }
    let output = rustc.arg(fixture).output().expect("run rustc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        return Err(format!("compiled, but must fail\n{stderr}"));
    }
    let file = fixture.file_name().and_then(|s| s.to_str()).unwrap_or(stem);
    let lines = directive("//@ line:")
        .into_iter()
        .map(|line| format!("{file}:{line}:"));
    let missing: Vec<String> = errors
        .into_iter()
        .map(str::to_owned)
        .chain(lines)
        .filter(|needle| !stderr.contains(needle.as_str()))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("output lacks {missing:?}\n{stderr}"))
    }
}

#[test]
fn every_fixture_fails_with_its_diagnostics() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("compile-fail");
    let library = library(&scratch);
    let mut all = Vec::new();
    fixtures(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compile_fail"),
        &mut all,
    );
    all.sort();
    assert!(!all.is_empty(), "no fixtures found");
    let queue = Mutex::new(all.iter());
    let failures = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().expect("queue").next();
                    let Some(fixture) = next else { break };
                    if let Err(why) = check(fixture, &library, &scratch) {
                        failures
                            .lock()
                            .expect("failures")
                            .push(format!("{}: {why}", fixture.display()));
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().expect("failures");
    failures.sort();
    assert!(
        failures.is_empty(),
        "{} of {} fixtures missed:\n\n{}",
        failures.len(),
        all.len(),
        failures.join("\n\n")
    );
}
