//! Regression: write-begin → `EINVAL` (os error 22), process-local, on
//! macOS — struck the primer graph-builder's production store twice on
//! 2026-07-17 (a 3.4 h writer mid-run, and a fresh reads-only process on
//! its FIRST write), while fresh probe processes on the same store
//! succeeded throughout.
//!
//! Root cause: heed's vendored LMDB, built without `posix-sem`, lands on
//! `MDB_USE_SYSV_SEM` on Apple platforms. The reader/writer locks are
//! then a `SysV` semaphore set keyed by `ftok(lock.mdb, 'M')`, and Darwin's
//! `ftok` keeps only the low 16 bits of the lockfile inode (plus the low
//! 8 of the device) — every LMDB environment on the volume whose
//! lockfile inode collides mod 2^16 shares the set. A colliding
//! environment resets the semaphores at its open (`SETALL {1,1}`) and
//! REMOVES the set at its close (`semctl(IPC_RMID)`, taken under the
//! exclusive lock on its OWN lockfile, which nothing contests). The
//! long-lived process keeps the `semid` it cached at open, so its next
//! `semop` — write-begin's writer lock — returns `EINVAL`, while any
//! fresh process re-`semget`s a new set and finds a healthy store. Reads
//! can keep working past the removal (the engine's parked reader skips
//! the reader-slot `semop`), so the first WRITE is where it surfaces.
//! On a volume with heavy file churn the inode counter sweeps the 2^16
//! space, so any other LMDB user on the machine (the engine's own test
//! and fuzz lanes included) eventually mints a colliding lockfile.
//!
//! This test distills the collision to its kernel mechanism: remove the
//! open store's semaphore set exactly as a colliding close would, then
//! require write-begin to survive. Under the fix (heed's `posix-sem` on
//! macOS) there is no `SysV` set to remove — POSIX named semaphores stay
//! valid in-process even after `sem_unlink`, so the whole
//! remote-invalidation class is gone, not just improbable.
#![cfg(target_os = "macos")]

mod common;

use std::os::unix::fs::MetadataExt as _;

use bumbledb::Db;

bumbledb::schema! {
    pub Tiny;

    relation Row {
        id: u64 as RowId, fresh,
        val: i64,
    }
}

#[test]
fn write_begin_survives_a_colliding_sysv_semaphore_removal() {
    let dir = common::TempDir::new("sysv-sem-rmid");
    let db = Db::create(dir.path(), Tiny).expect("create");
    db.write(|_| Ok(())).expect("the pre-removal write");

    // Darwin ftok(path, 'M'): ('M' << 24) | ((dev & 0xff) << 16) | (ino & 0xffff).
    let meta = std::fs::metadata(dir.path().join("lock.mdb")).expect("LMDB's lockfile exists");
    let dev_byte = u32::try_from(meta.dev() & 0xff).expect("masked to a byte");
    let ino_low = u32::try_from(meta.ino() & 0xffff).expect("masked to 16 bits");
    let key = (u32::from(b'M') << 24) | (dev_byte << 16) | ino_low;

    // The colliding environment's close, distilled: remove the SysV set
    // at the store's key. A failure exit means no set exists under the
    // key — the fixed (posix-sem) build's expected state.
    let removed = std::process::Command::new("/usr/bin/ipcrm")
        .args(["-S", &key.to_string()])
        .output()
        .expect("ipcrm runs");

    db.write(|_| Ok(())).unwrap_or_else(|err| {
        panic!(
            "write begin after external semaphore removal \
             (a SysV set existed and was removed: {}): {err}",
            removed.status.success()
        )
    });
}
