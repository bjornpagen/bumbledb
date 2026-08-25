//! Fresh-id leases: birth claims `[0, 4096)`, CAS increments lease
//! disjoint blocks, `Moved` retries unbounded, ids ride in commands as
//! plain values, and the counter object is the failover floor an
//! adopting writer reads. Cross-process disjointness runs against real
//! child processes, in the lane-B tradition.

mod lane_e_support;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use bumbledb::schema::FieldId;
use bumbledb::{SchemaDescriptor, Value};
use bumbledb_log::lease::{LEASE_WIDTH, LeaseRefusal, Leased, ids_key, lease_block};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Error, Options, Writer, WriterOpened};
use lane_e_support::{NOTE, codec, note_braid, temp_dir, theory};

const ROLE_ENV: &str = "LANE_E_CHILD_ROLE";
const BASE_ENV: &str = "LANE_E_BASE_DIR";
const ID_ENV: &str = "LANE_E_CHILD_ID";
const CHILDREN: u64 = 6;
const BLOCKS_PER_CHILD: u64 = 8;

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
    assert!(matches!(outcome, Commit::Accepted { generation: 1, .. }));

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
        .expect("commit");
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
        .expect("commit");
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
        .expect("commit");
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
    for _ in 0..BLOCKS_PER_CHILD {
        match lease_block(&store, "", NOTE, FieldId(0), 0, 1).expect("lease") {
            Leased::Drawn { range, .. } => {
                println!(
                    "LANE_E lease id={id} start={} end={}",
                    range.start, range.end
                );
            }
            Leased::Refused(refusal) => panic!("lease refused: {refusal:?}"),
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
    let mut children: Vec<Child> = Vec::new();
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn child");
        children.push(child);
    }

    let mut ranges: Vec<(u64, u64)> = Vec::new();
    for child in children {
        let out = child.wait_with_output().expect("child exit");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            out.status.success(),
            "child failed: {stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        for line in stdout.lines() {
            let Some(at) = line.find("LANE_E lease ") else {
                continue;
            };
            let mut start = None;
            let mut end = None;
            for token in line[at..].split_whitespace() {
                if let Some(v) = token.strip_prefix("start=") {
                    start = Some(v.parse::<u64>().expect("start"));
                }
                if let Some(v) = token.strip_prefix("end=") {
                    end = Some(v.parse::<u64>().expect("end"));
                }
            }
            ranges.push((start.expect("start"), end.expect("end")));
        }
    }
    assert_eq!(ranges.len() as u64, CHILDREN * BLOCKS_PER_CHILD);
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        assert!(
            pair[0].1 <= pair[1].0,
            "cross-writer collision is structurally impossible: {pair:?}"
        );
    }
    let store = FsStore::new(base.join("store"));
    let counter = store
        .get(&ids_key("", NOTE, FieldId(0)))
        .expect("get")
        .expect("counter");
    assert_eq!(
        counter.bytes,
        (CHILDREN * BLOCKS_PER_CHILD * LEASE_WIDTH)
            .to_string()
            .into_bytes(),
        "every block is accounted"
    );
}
