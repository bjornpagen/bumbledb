//! Regression: write-begin → `EINVAL` (os error 22), process-local, on
//! macOS — struck the primer graph-builder's production store twice on
//! 2026-07-17 (a 3.4 h writer mid-run, and a fresh reads-only process on
#![cfg(target_os = "macos")]
mod common;

use std::os::unix::fs::MetadataExt as _;

use bumbledb::Db;

bumbledb::schema! {
    pub Tiny;

    relation Row {
        id: u64 as RowId,
        val: i64,
    }
}

#[test]
fn write_begin_survives_a_colliding_sysv_semaphore_removal() {
    let dir = common::TempDir::new("sysv-sem-rmid");
    let db = Db::create(dir.path(), Tiny)
        .expect("create")
        .expect("accepted");
    db.write(|_| Ok(()))
        .expect("the pre-removal write")
        .unwrap();

    // Darwin ftok(path, 'M'): ('M' << 24) | ((dev & 0xff) << 16) | (ino & 0xffff).
    let meta = std::fs::metadata(dir.path().join("lock.mdb")).expect("LMDB's lockfile exists");
    let dev_byte = u32::try_from(meta.dev() & 0xff).expect("masked to a byte");
    let ino_low = u32::try_from(meta.ino() & 0xffff).expect("masked to 16 bits");
    let key = (u32::from(b'M') << 24) | (dev_byte << 16) | ino_low;

    // The colliding environment's close, distilled: remove the SysV set

    // key — the fixed (posix-sem) build's expected state.
    let removed = std::process::Command::new("/usr/bin/ipcrm")
        .args(["-S", &key.to_string()])
        .output()
        .expect("ipcrm runs");

    db.write(|_| Ok(()))
        .unwrap_or_else(|err| {
            panic!(
                "write begin after external semaphore removal \
             (a SysV set existed and was removed: {}): {err}",
                removed.status.success()
            )
        })
        .unwrap();
}
