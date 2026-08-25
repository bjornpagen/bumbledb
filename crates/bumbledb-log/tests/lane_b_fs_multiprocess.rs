//! Multi-process soundness of `FsStore` arbitration: real child
//! processes (this test binary re-executed with a role env var) hammer
//! one slot key with `put_create` and one manifest key with `put_swap`.
//! Exactly one creator wins the slot; every swap linearizes and every
//! loser gets `Moved`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bumbledb_log::store::fs::{content_etag, FsStore};
use bumbledb_log::store::{Create, ObjectStore, StoreKey, Swap};

const ROLE_ENV: &str = "LANE_B_CHILD_ROLE";
const BASE_ENV: &str = "LANE_B_BASE_DIR";
const ID_ENV: &str = "LANE_B_CHILD_ID";

const SLOT_KEY: &str = "log/c00000001/slot";
const MANIFEST_KEY: &str = "manifest.json";
const WRITERS: u64 = 8;
const SWAPS_PER_WRITER: u64 = 16;

fn base_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lane_b_mp_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create base dir");
    dir
}

fn store_at(base: &Path) -> FsStore {
    FsStore::new(base.join("store"))
}

fn go_path(base: &Path) -> PathBuf {
    base.join("go")
}

fn wait_for_go(base: &Path) {
    let go = go_path(base);
    let deadline = Instant::now() + Duration::from_secs(20);
    while !go.exists() {
        assert!(Instant::now() < deadline, "start barrier never appeared");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn slot_body(id: u64) -> Vec<u8> {
    format!("writer {id} claims the slot").into_bytes()
}

fn spawn_children(test_name: &str, role: &str, base: &Path) -> Vec<Child> {
    let exe = std::env::current_exe().expect("current test binary");
    let mut children = Vec::new();
    for id in 0..WRITERS {
        let child = Command::new(&exe)
            .args([test_name, "--exact", "--nocapture", "--test-threads=1"])
            .env(ROLE_ENV, role)
            .env(BASE_ENV, base.as_os_str())
            .env(ID_ENV, id.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn child writer");
        children.push(child);
    }
    children
}

fn harvest(children: Vec<Child>) -> Vec<String> {
    let mut reports = Vec::new();
    for child in children {
        let out = child.wait_with_output().expect("child exit");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            out.status.success(),
            "child writer failed: {stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // The child harness prints its own "test name ..." prefix on the
        // same line as the report, so the marker is found, not anchored.
        reports.extend(
            stdout
                .lines()
                .filter_map(|line| line.find("LANE_B ").map(|at| line[at..].to_string())),
        );
    }
    reports
}

fn child_env() -> Option<(String, PathBuf, u64)> {
    let role = std::env::var(ROLE_ENV).ok()?;
    let base = PathBuf::from(std::env::var_os(BASE_ENV).expect("base dir env"));
    let id = std::env::var(ID_ENV)
        .expect("child id env")
        .parse::<u64>()
        .expect("child id parses");
    Some((role, base, id))
}

fn run_create_child(base: &Path, id: u64) {
    let store = store_at(base);
    wait_for_go(base);
    let key = StoreKey::of(SLOT_KEY);
    let body = slot_body(id);
    let outcome = store.put_create(&key, &body).expect("put_create");
    let word = match outcome {
        Create::Created(_) => "created",
        Create::Exists => "exists",
        Create::Ambiguous => match store.get(&key).expect("verify") {
            Some(fetched) if fetched.bytes == body => "created",
            Some(_) => "exists",
            None => panic!("Ambiguous create left no occupant"),
        },
    };
    println!("LANE_B create id={id} outcome={word}");
}

fn run_swap_child(base: &Path, id: u64) {
    let store = store_at(base);
    // Deterministic loser: a swap under a stale etag must come back
    // Moved before the contention loop even starts.
    let stale = content_etag(b"not the manifest at all");
    let lost = store
        .put_swap(&StoreKey::of(MANIFEST_KEY), b"usurper", &stale)
        .expect("stale swap");
    assert_eq!(lost, Swap::Moved, "stale etag must lose as Moved");

    wait_for_go(base);
    let mut swapped = 0u64;
    let mut moved = 0u64;
    while swapped < SWAPS_PER_WRITER {
        let current = store
            .get(&StoreKey::of(MANIFEST_KEY))
            .expect("get manifest")
            .expect("manifest present");
        let value: u64 = String::from_utf8(current.bytes)
            .expect("manifest utf8")
            .parse()
            .expect("manifest decimal");
        let next = (value + 1).to_string();
        match store
            .put_swap(&StoreKey::of(MANIFEST_KEY), next.as_bytes(), &current.etag)
            .expect("put_swap")
        {
            Swap::Swapped(_) => swapped += 1,
            Swap::Moved => moved += 1,
            Swap::Ambiguous => {}
        }
    }
    println!("LANE_B swap id={id} swapped={swapped} moved={moved}");
}

#[test]
fn slot_create_is_won_exactly_once_across_processes() {
    if let Some((role, base, id)) = child_env() {
        if role == "create" {
            run_create_child(&base, id);
        }
        return;
    }

    let base = base_dir("create");
    let children = spawn_children(
        "slot_create_is_won_exactly_once_across_processes",
        "create",
        &base,
    );
    std::fs::write(go_path(&base), b"go").expect("raise barrier");
    let reports = harvest(children);

    assert_eq!(
        reports.len() as u64,
        WRITERS,
        "one report per writer: {reports:?}"
    );
    let winners: Vec<u64> = reports
        .iter()
        .filter(|line| line.ends_with("outcome=created"))
        .map(|line| {
            line.split_whitespace()
                .find_map(|tok| tok.strip_prefix("id="))
                .expect("winner id field")
                .parse()
                .expect("winner id parses")
        })
        .collect();
    assert_eq!(winners.len(), 1, "exactly one Created: {reports:?}");
    let losers = reports
        .iter()
        .filter(|line| line.ends_with("outcome=exists"))
        .count() as u64;
    assert_eq!(
        losers,
        WRITERS - 1,
        "every other writer gets Exists: {reports:?}"
    );

    let fetched = store_at(&base)
        .get(&StoreKey::of(SLOT_KEY))
        .expect("get slot")
        .expect("slot present");
    assert_eq!(
        fetched.bytes,
        slot_body(winners[0]),
        "the slot holds the winner's bytes, untouched by losers"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn manifest_swaps_linearize_across_processes() {
    if let Some((role, base, id)) = child_env() {
        if role == "swap" {
            run_swap_child(&base, id);
        }
        return;
    }

    let base = base_dir("swap");
    let store = store_at(&base);
    assert!(matches!(
        store
            .put_create(&StoreKey::of(MANIFEST_KEY), b"0")
            .expect("manifest birth"),
        Create::Created(_)
    ));

    let children = spawn_children("manifest_swaps_linearize_across_processes", "swap", &base);
    std::fs::write(go_path(&base), b"go").expect("raise barrier");
    let reports = harvest(children);

    assert_eq!(
        reports.len() as u64,
        WRITERS,
        "one report per writer: {reports:?}"
    );
    for line in &reports {
        assert!(
            line.contains(&format!("swapped={SWAPS_PER_WRITER} ")),
            "every writer lands all its swaps: {line}"
        );
    }

    let fetched = store
        .get(&StoreKey::of(MANIFEST_KEY))
        .expect("get manifest")
        .expect("manifest present");
    let total: u64 = String::from_utf8(fetched.bytes)
        .expect("manifest utf8")
        .parse()
        .expect("manifest decimal");
    assert_eq!(
        total,
        WRITERS * SWAPS_PER_WRITER,
        "no swap was lost and none applied twice"
    );
    let _ = std::fs::remove_dir_all(&base);
}
