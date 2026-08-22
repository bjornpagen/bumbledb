//! consistent committed state — LMDB atomicity exercised, not trusted.
use std::process::{Command, Stdio};
use std::time::Duration;

use bumbledb::Db;

mod common;

bumbledb::schema! {
    pub Store;

    relation Item {
        id: u64 as ItemId, fresh,
        seq: u64,
    }
}

fn item(k: u64) -> Item {
    Item {
        id: ItemId(k),
        seq: k * 7,
    }
}

#[test]
#[ignore = "crash-child body; spawned by kill_during_commit_leaves_a_consistent_database"]
fn crash_child_commit_loop() {
    let Ok(dir) = std::env::var("BUMBLEDB_CRASH_DIR") else {
        return;
    };
    let db = Db::open(std::path::Path::new(&dir), Store).expect("child open");
    for k in 1..u64::MAX {
        db.write(|tx| {
            tx.insert([&item(k)])?;
            if k > 1 {
                tx.delete([&item(k - 1)])?;
            }
            Ok(())
        })
        .expect("child write")
        .unwrap();
    }
}

#[test]
fn kill_during_commit_leaves_a_consistent_database() {
    let exe = std::env::current_exe().expect("test binary path");
    for (round, delay_ms) in [5u64, 20, 60].into_iter().enumerate() {
        let dir = common::TempDir::new(&format!("crash-{round}"));
        drop(
            Db::create(dir.path(), Store)
                .expect("create")
                .expect("accepted"),
        );

        let mut child = Command::new(&exe)
            .args([
                "crash_child_commit_loop",
                "--exact",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BUMBLEDB_CRASH_DIR", dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn child");
        std::thread::sleep(Duration::from_millis(delay_ms));
        child.kill().expect("SIGKILL");
        let _ = child.wait();

        // Reopen: format + fingerprint verify, then sweep consistency

        let db = Db::open(dir.path(), Store).expect("open after crash");
        let live: Vec<Item> = db
            .read(|snap| snap.scan_facts::<Item>()?.collect())
            .expect("scan after crash");
        assert!(
            live.len() <= 1,
            "round {round}: every committed state holds at most one item, found {live:?}"
        );
        let max_seen = live.first().map_or(0, |i| i.id.0);
        if let Some(item_k) = live.first() {
            assert_eq!(item_k.seq, item_k.id.0 * 7, "round {round}: torn fact");
        }

        db.write(|tx| {
            if let Some(existing) = live.first() {
                assert_eq!(
                    tx.insert([existing])?.changed(),
                    0,
                    "round {round}: committed fact not visible to membership"
                );
            }

            let next: ItemId = tx.reserve(1)?.start().expect("nonempty");
            assert!(
                next.0 > max_seen || live.is_empty(),
                "round {round}: fresh {next:?} at or below committed {max_seen}"
            );
            tx.insert([&item(next.0)])?;
            Ok(())
        })
        .expect("write after crash")
        .unwrap();

        let count = db
            .read(|snap| Ok(snap.scan_facts::<Item>()?.count()))
            .expect("count after crash");
        assert!(count >= 1);
    }
}

#[test]
#[ignore = "crash-child body; spawned by kill_during_counters_only_commit_leaves_q_consistent"]
fn crash_child_reserve_loop() {
    let Ok(dir) = std::env::var("BUMBLEDB_CRASH_RESERVE_DIR") else {
        return;
    };
    let db = Db::open(std::path::Path::new(&dir), Store).expect("child open");
    for _ in 0..u64::MAX {
        db.write(|tx| {
            let _: ItemId = tx.reserve(1)?.start().expect("nonempty");
            Ok(())
        })
        .expect("child reserve")
        .unwrap();
    }
}

#[test]
fn kill_during_counters_only_commit_leaves_q_consistent() {
    let exe = std::env::current_exe().expect("test binary path");
    for (round, delay_ms) in [10u64, 40].into_iter().enumerate() {
        let dir = common::TempDir::new(&format!("crash-reserve-{round}"));
        drop(
            Db::create(dir.path(), Store)
                .expect("create")
                .expect("accepted"),
        );

        let mut child = Command::new(&exe)
            .args([
                "crash_child_reserve_loop",
                "--exact",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BUMBLEDB_CRASH_RESERVE_DIR", dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn child");
        std::thread::sleep(Duration::from_millis(delay_ms));
        child.kill().expect("SIGKILL");
        let _ = child.wait();

        let db = Db::open(dir.path(), Store).expect("open after crash");

        let count = db
            .read(|snap| Ok(snap.scan_facts::<Item>()?.count()))
            .expect("scan after crash");
        assert_eq!(count, 0, "round {round}: reserve-only child wrote a fact");
        assert_eq!(
            db.generation().expect("generation").value(),
            0,
            "round {round}: a counters-only commit moved the generation"
        );

        let a: ItemId = db
            .write(|tx| Ok(tx.reserve(1)?.start().expect("nonempty")))
            .expect("reserve after crash")
            .unwrap()
            .value;
        let b: ItemId = db
            .write(|tx| Ok(tx.reserve(1)?.start().expect("nonempty")))
            .expect("reserve after crash")
            .unwrap()
            .value;
        assert_eq!(b.0, a.0 + 1, "round {round}: Q mark torn or regressed");

        db.write(|tx| {
            let id: ItemId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&item(id.0)]).map(|_| ())
        })
        .expect("write after crash")
        .unwrap();
    }
}
