//! Fresh-id leases: birth claims `[0, 4096)`, CAS increments lease
//! disjoint blocks, `Moved` retries unbounded, ids ride in commands as
//! plain values, and the counter object is the failover floor an
//! adopting writer reads. Cross-process disjointness runs against real
//! child processes, in the lane-B tradition. Bounded local lock contention
//! is not a promise of per-process fairness: every attempted lease is
//! accounted as either drawn or the exact pre-mutation wait exhaustion.
//! There are no retries of failed calls or relaxed disjointness assertions.

mod lane_e_support;

use std::collections::BTreeSet;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bumbledb::schema::FieldId;
use bumbledb::{Admission, SchemaDescriptor, Value};
use bumbledb_log::lease::{LEASE_WIDTH, LeaseRefusal, Leased, ids_key, lease_block};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{ObjectStore, StoreError, StoreKey};
use bumbledb_log::writer::{Error, Options, Slotted, Writer, WriterOpened};
use lane_e_support::{NOTE, codec, note_braid, temp_dir, theory};

const ROLE_ENV: &str = "LANE_E_CHILD_ROLE";
const BASE_ENV: &str = "LANE_E_BASE_DIR";
const ID_ENV: &str = "LANE_E_CHILD_ID";
const CHILDREN: u64 = 6;
const BLOCKS_PER_CHILD: u64 = 8;
const CHILD_EXIT_WAIT: Duration = Duration::from_secs(60);

fn ready<S: bumbledb_log::store::ObjectStore + 'static>(
    opened: WriterOpened<SchemaDescriptor, S>,
) -> Writer<SchemaDescriptor, S> {
    match opened {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn reserve_draws_from_birth_and_advances_the_counter() {
    let root = temp_dir("draw");
    let dir = root.join("w");
    let writer = ready(
        Writer::open(
            FsStore::new(root.clone()),
            "",
            &dir,
            theory(),
            Options::new(61),
        )
        .expect("open writer"),
    );

    let outcome = writer
        .commit(|batch| {
            let ids = batch.reserve(NOTE, FieldId(0), 3)?;
            assert_eq!(ids, 0..3, "birth claims the first block");
            for id in ids {
                batch.insert(
                    NOTE,
                    [Box::from([Value::U64(id), Value::String("n".into())])],
                );
            }
            Ok(())
        })
        .expect("commit");
    assert!(matches!(
        outcome,
        Admission::Accepted(Slotted { slot: 1, .. })
    ));

    let store = FsStore::new(root.clone());
    let counter = store
        .get(&ids_key("", NOTE, FieldId(0)))
        .expect("get")
        .expect("counter object born");
    assert_eq!(counter.bytes, b"4096");

    // Ids ride in commands as plain values.
    let codec = codec();
    let braid = note_braid(&codec);
    let slot = store
        .get(&bumbledb_log::manifest::log_key("", braid, 1))
        .expect("get")
        .expect("published");
    let batch = codec.decode(&slot.bytes).expect("decode");
    assert_eq!(batch.ops[0].rows[0][0], Value::U64(0));

    // The cached block keeps serving locally; outgrowing it leases a
    // fresh block — unique, never dense.
    writer
        .commit(|batch| {
            let more = batch.reserve(NOTE, FieldId(0), 4_090)?;
            assert_eq!(more, 3..4_093);
            let tail = batch.reserve(NOTE, FieldId(0), 10)?;
            assert_eq!(tail, 4_096..4_106, "the 3-id remainder is abandoned");
            batch.insert(
                NOTE,
                [Box::from([
                    Value::U64(tail.start),
                    Value::String("t".into()),
                ])],
            );
            Ok(())
        })
        .expect("commit")
        .unwrap();
    let counter = store
        .get(&ids_key("", NOTE, FieldId(0)))
        .expect("get")
        .expect("counter");
    assert_eq!(counter.bytes, b"8192");
}

#[test]
fn adoption_reads_the_counter_as_the_floor() {
    let root = temp_dir("floor");
    let dir_a = root.join("a");
    let dir_b = root.join("b");
    let writer_a = ready(
        Writer::open(
            FsStore::new(root.clone()),
            "",
            &dir_a,
            theory(),
            Options::new(1),
        )
        .expect("open writer"),
    );
    writer_a
        .commit(|batch| {
            let ids = batch.reserve(NOTE, FieldId(0), 1)?;
            batch.insert(
                NOTE,
                [Box::from([
                    Value::U64(ids.start),
                    Value::String("a".into()),
                ])],
            );
            Ok(())
        })
        .expect("commit")
        .unwrap();
    drop(writer_a);

    let writer_b = ready(
        Writer::open(FsStore::new(root), "", &dir_b, theory(), Options::new(2))
            .expect("open writer"),
    );
    writer_b
        .commit(|batch| {
            let ids = batch.reserve(NOTE, FieldId(0), 1)?;
            assert_eq!(
                ids,
                4_096..4_097,
                "the counter object is the failover floor"
            );
            batch.insert(
                NOTE,
                [Box::from([
                    Value::U64(ids.start),
                    Value::String("b".into()),
                ])],
            );
            Ok(())
        })
        .expect("commit")
        .unwrap();
}

#[test]
fn malformed_counter_and_over_width_refuse() {
    let root = temp_dir("refuse");
    let dir = root.join("w");
    let store = FsStore::new(root.clone());
    let key = ids_key("", NOTE, FieldId(0));
    store.put_create(&key, b"007").expect("plant malformed");

    let writer = ready(
        Writer::open(FsStore::new(root), "", &dir, theory(), Options::new(62))
            .expect("open writer"),
    );
    let err = writer
        .commit::<()>(|batch| {
            batch.reserve(NOTE, FieldId(0), 1)?;
            Ok(())
        })
        .expect_err("malformed counter refuses");
    assert!(matches!(
        err,
        Error::Lease(LeaseRefusal::Counter { relation, .. }) if relation == NOTE
    ));

    let err = writer
        .commit::<()>(|batch| {
            batch.reserve(NOTE, FieldId(1), LEASE_WIDTH + 1)?;
            Ok(())
        })
        .expect_err("a draw beyond one lease width refuses");
    assert!(matches!(
        err,
        Error::Lease(LeaseRefusal::OverWidth { requested }) if requested == LEASE_WIDTH + 1
    ));
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

fn run_lease_child(base: &Path, id: u64) {
    let store = FsStore::new(base.join("store"));
    let key = ids_key("", NOTE, FieldId(0));
    let mut start = String::new();
    assert!(
        io::stdin().read_line(&mut start).unwrap() > 0,
        "parent start gate"
    );
    assert_eq!(start, "go\n");
    for attempt in 0..BLOCKS_PER_CHILD {
        match lease_block(&store, "", NOTE, FieldId(0), 0, 1) {
            Ok(Leased::Drawn { range, .. }) => {
                println!(
                    "LANE_E attempt id={id} attempt={attempt} drawn={}..{}",
                    range.start, range.end
                );
            }
            Err(error) if mutation_wait_exhausted(&error, &key) => {
                println!("LANE_E attempt id={id} attempt={attempt} busy");
            }
            Err(error) => panic!("lease failed: {error:?}"),
            Ok(Leased::Refused(refusal)) => panic!("lease refused: {refusal:?}"),
        }
    }
}

// Only this local adapter's known pre-mutation lock refusal is admissible.
// Other WouldBlock errors, infrastructure failures, and protocol refusals must
// still fail the test. No ambiguous write is relabeled as an empty attempt.
fn mutation_wait_exhausted(error: &StoreError, key: &StoreKey) -> bool {
    matches!(error.op, "put_create" | "put_swap")
        && error.key == key.as_str()
        && error.source.kind() == io::ErrorKind::WouldBlock
        && error.source.to_string() == "local mutation wait exhausted"
}

#[test]
fn contention_classification_does_not_hide_other_store_failures() {
    let key = ids_key("", NOTE, FieldId(0));
    let error = |op, key: &str, kind, message| StoreError {
        op,
        key: key.to_string(),
        source: io::Error::new(kind, message),
    };
    for op in ["put_create", "put_swap"] {
        assert!(mutation_wait_exhausted(
            &error(
                op,
                key.as_str(),
                io::ErrorKind::WouldBlock,
                "local mutation wait exhausted"
            ),
            &key
        ));
    }
    for (op, name, kind, message) in [
        (
            "get",
            key.as_str(),
            io::ErrorKind::WouldBlock,
            "local mutation wait exhausted",
        ),
        (
            "put_swap",
            "other/key",
            io::ErrorKind::WouldBlock,
            "local mutation wait exhausted",
        ),
        (
            "put_swap",
            key.as_str(),
            io::ErrorKind::PermissionDenied,
            "local mutation wait exhausted",
        ),
        (
            "put_swap",
            key.as_str(),
            io::ErrorKind::WouldBlock,
            "unclassified storage failure",
        ),
    ] {
        assert!(!mutation_wait_exhausted(
            &error(op, name, kind, message),
            &key
        ));
    }
}

struct LeaseChildren(Vec<Child>);

impl LeaseChildren {
    fn reap_all(&mut self) {
        let started = Instant::now();
        loop {
            let mut exited = true;
            for child in &mut self.0 {
                exited &= child.try_wait().expect("poll owned child").is_some();
            }
            if exited {
                return;
            }
            assert!(
                started.elapsed() < CHILD_EXIT_WAIT,
                "lease children did not exit"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for LeaseChildren {
    fn drop(&mut self) {
        // Kill all before waiting: a failed spawn/assertion must not orphan
        // a sibling holding the mutation lock or captured output handles.
        for child in &mut self.0 {
            let _ = child.kill();
        }
        for child in &mut self.0 {
            let _ = child.wait();
        }
    }
}

fn read_attempts(
    stdout: &str,
    id: usize,
    attempts: &mut BTreeSet<(u64, u64)>,
    ranges: &mut Vec<(u64, u64)>,
    busy: &mut u64,
) {
    for line in stdout.lines() {
        let Some(at) = line.find("LANE_E attempt ") else {
            continue;
        };
        let event: Vec<_> = line[at + "LANE_E attempt ".len()..]
            .split_whitespace()
            .collect();
        assert_eq!(event.len(), 3, "malformed event {line}");
        let reporter = event[0]
            .strip_prefix("id=")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert_eq!(
            reporter, id as u64,
            "a child cannot report another's attempt"
        );
        let attempt = event[1]
            .strip_prefix("attempt=")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert!(attempt < BLOCKS_PER_CHILD);
        assert!(
            attempts.insert((reporter, attempt)),
            "duplicate attempt {line}"
        );
        if event[2] == "busy" {
            *busy += 1;
        } else {
            let (start, end) = event[2]
                .strip_prefix("drawn=")
                .unwrap()
                .split_once("..")
                .unwrap();
            ranges.push((start.parse().unwrap(), end.parse().unwrap()));
        }
    }
}

#[test]
fn leases_are_disjoint_across_processes() {
    if let Some((role, base, id)) = child_env() {
        if role == "lease" {
            run_lease_child(&base, id);
        }
        return;
    }

    let base = temp_dir("mp");
    let exe = std::env::current_exe().expect("current test binary");
    let mut children = LeaseChildren(Vec::new());
    for id in 0..CHILDREN {
        let child = Command::new(&exe)
            .args([
                "leases_are_disjoint_across_processes",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(ROLE_ENV, "lease")
            .env(BASE_ENV, base.as_os_str())
            .env(ID_ENV, id.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn child");
        children.0.push(child);
    }
    // All six processes are spawned before any may attempt an allocation.
    for child in &mut children.0 {
        child.stdin.take().unwrap().write_all(b"go\n").unwrap();
    }
    children.reap_all();

    let mut ranges: Vec<(u64, u64)> = Vec::new();
    let mut attempts = BTreeSet::new();
    let mut busy = 0_u64;
    for (id, child) in children.0.iter_mut().enumerate() {
        // Each child emits at most eight short events. All children have been
        // reaped before reading/checking output or attempting the final lease.
        let mut stdout = String::new();
        let mut stderr = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(
            child.wait().unwrap().success(),
            "child failed: {stdout}\n{stderr}"
        );
        read_attempts(&stdout, id, &mut attempts, &mut ranges, &mut busy);
    }
    drop(children);
    assert_eq!(attempts.len() as u64, CHILDREN * BLOCKS_PER_CHILD);
    assert_eq!(ranges.len() as u64 + busy, CHILDREN * BLOCKS_PER_CHILD);
    assert!(
        !ranges.is_empty(),
        "contended allocations must not be vacuous"
    );
    println!(
        "LANE_E accounted: {} drawn, {busy} bounded contention",
        ranges.len()
    );
    ranges.sort_unstable();
    for (index, &(start, end)) in ranges.iter().enumerate() {
        assert_eq!(
            start,
            index as u64 * LEASE_WIDTH,
            "no gap or overlapping lease"
        );
        assert_eq!(
            end,
            start + LEASE_WIDTH,
            "every successful lease has exact width"
        );
    }
    let store = FsStore::new(base.join("store"));
    let Leased::Drawn { range: tail, .. } =
        lease_block(&store, "", NOTE, FieldId(0), 0, 1).expect("uncontended tail lease")
    else {
        panic!("uncontended tail refused");
    };
    assert_eq!(tail.start, ranges.len() as u64 * LEASE_WIDTH);
    assert_eq!(tail.end, tail.start + LEASE_WIDTH);
    let counter = store
        .get(&ids_key("", NOTE, FieldId(0)))
        .expect("get")
        .expect("counter");
    assert_eq!(
        counter.bytes,
        tail.end.to_string().into_bytes(),
        "every successful block including the uncontended tail is accounted; busy never allocates"
    );
}
