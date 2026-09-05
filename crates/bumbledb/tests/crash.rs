//! consistent committed state — LMDB atomicity exercised, not trusted.
use std::process::{Command, Stdio};
use std::time::Duration;

use bumbledb::Db;

mod common;

bumbledb::schema! {
    pub Store;

    relation Item {
        id: u64 as ItemId,
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
    let db = Db::open(std::path::Path::new(&dir), Store, common::work()).expect("child open");
    for k in 1..u64::MAX {
        db.write(common::work(), |tx| {
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
            Db::create(dir.path(), Store, common::work())
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

        let db = Db::open(dir.path(), Store, common::work()).expect("open after crash");
        let live: Vec<Item> = db
            .read(common::work(), |snap| snap.scan_facts::<Item>()?.collect())
            .expect("scan after crash");
        assert!(
            live.len() <= 1,
            "round {round}: every committed state holds at most one item, found {live:?}"
        );
        let max_seen = live.first().map_or(0, |i| i.id.0);
        if let Some(item_k) = live.first() {
            assert_eq!(item_k.seq, item_k.id.0 * 7, "round {round}: torn fact");
        }

        db.write(common::work(), |tx| {
            if let Some(existing) = live.first() {
                assert_eq!(
                    tx.insert([existing])?.changed(),
                    0,
                    "round {round}: committed fact not visible to membership"
                );
            }

            // The database issues no identity: the application picks the
            // next id past everything it saw committed.
            let next = max_seen + 1;
            tx.insert([&item(next)])?;
            Ok(())
        })
        .expect("write after crash")
        .unwrap();

        let count = db
            .read(common::work(), |snap| Ok(snap.scan_facts::<Item>()?.count()))
            .expect("count after crash");
        assert!(count >= 1);
    }
}

// The counters-only crash tests (`crash_child_reserve_loop`,
// `kill_during_counters_only_commit_leaves_q_consistent`) retired with the
// fresh reservation machinery (E-NO-RESERVE): the successor has no Q
// counter and no counters-only commit to tear.
