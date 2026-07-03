//! Kill-during-commit crash injection (docs/architecture/50-validation.md):
//! a child process commits a known delta sequence in a loop, the parent
//! SIGKILLs it mid-flight, and the reopened database must be *some*
//! consistent committed state — LMDB atomicity exercised, not trusted.

use std::process::{Command, Stdio};
use std::time::Duration;

use bumbledb::Db;

bumbledb::schema! {
    relation Item {
        id: u64 as ItemId, serial,
        seq: u64,
    }
}

fn item(k: u64) -> Item {
    Item {
        id: ItemId(k),
        seq: k * 7,
    }
}

/// The child body: every committed state is exactly one live item (insert
/// `k`, delete `k-1` in one transaction), so any consistent post-crash
/// state is enumerable. Run only via the parent test below.
#[test]
#[ignore = "crash-child body; spawned by kill_during_commit_leaves_a_consistent_database"]
fn crash_child_commit_loop() {
    let Ok(dir) = std::env::var("BUMBLEDB_CRASH_DIR") else {
        return; // ran directly (e.g. `--ignored` sweeps): nothing to do
    };
    let db = Db::open(std::path::Path::new(&dir), schema()).expect("child open");
    for k in 1..u64::MAX {
        db.write(|tx| {
            tx.insert(&item(k))?;
            if k > 1 {
                tx.delete(&item(k - 1))?;
            }
            Ok(())
        })
        .expect("child write");
    }
}

#[test]
fn kill_during_commit_leaves_a_consistent_database() {
    let exe = std::env::current_exe().expect("test binary path");
    for (round, delay_ms) in [5u64, 20, 60].into_iter().enumerate() {
        let dir = std::env::temp_dir().join(format!("bumbledb-crash-{round}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test dir");
        drop(Db::create(&dir, schema()).expect("create"));

        let mut child = Command::new(&exe)
            .args([
                "crash_child_commit_loop",
                "--exact",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BUMBLEDB_CRASH_DIR", &dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn child");
        std::thread::sleep(Duration::from_millis(delay_ms));
        child.kill().expect("SIGKILL");
        let _ = child.wait();

        // Reopen: format + fingerprint verify, then sweep consistency
        // through the public surface.
        let db = Db::open(&dir, schema()).expect("open after crash");
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
            // M consistency: re-inserting the live fact is a no-op.
            if let Some(existing) = live.first() {
                assert!(
                    !tx.insert(existing)?,
                    "round {round}: committed fact not visible to membership"
                );
            }
            // Q consistency: the serial generator continues past every
            // committed id (a collision would corrupt the auto-unique).
            let next: ItemId = tx.alloc()?;
            assert!(
                next.0 > max_seen || live.is_empty(),
                "round {round}: serial {next:?} at or below committed {max_seen}"
            );
            tx.insert(&item(next.0))?;
            Ok(())
        })
        .expect("write after crash");

        // S consistency: image build cross-checks the stored row count
        // against a fresh F scan (RowCountMismatch is a hard error).
        let count = db
            .read(|snap| Ok(snap.scan_facts::<Item>()?.count()))
            .expect("count after crash");
        assert!(count >= 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
