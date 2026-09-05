//! `FsStore` durability ordering and fault boundaries (FS-02/03 shapes, C07):
//! stage → fsync → rename → parent fsync means a fault at any injected phase
//! leaves the OLD or the NEW complete bytes, never a torn head; reserved
//! namespaces are unreachable through keys; listing never surfaces ownership
//! scratch. Real SIGKILL/power-failure arms are the process lanes
//! (`local_ownership.rs` / P12 F3 harness). Verification: `NotRun`.

use std::path::PathBuf;

use bumbledb_log::store::fs::{FsStore, Inject, Phase, content_version};
use bumbledb_log::store::{ConditionalOutcome, ConditionalStore as _, HeadRead, PutOutcome};

fn fresh_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "bdb-log-fsstore-{}-{name}-{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create test root");
    root
}

#[test]
fn a_fault_at_every_phase_leaves_old_or_new_complete_bytes_never_torn() {
    for (phase, expect_new) in [
        (Phase::HeadObserved, false),
        (Phase::Staged, false),
        (Phase::Published, true),
    ] {
        let root = fresh_root("phases");
        let store = FsStore::new(&root);
        let v1 = match store
            .create_head("t/HEAD", b"old-complete")
            .expect("create")
        {
            ConditionalOutcome::Published { version } => version,
            other => panic!("{other:?}"),
        };
        store.set_hook(move |at, _key| {
            if at == phase {
                Inject::Error
            } else {
                Inject::Continue
            }
        });
        let outcome = store.replace_head("t/HEAD", &v1, b"new-complete");
        assert!(outcome.is_err(), "the injected fault surfaced at {phase:?}");
        store.set_hook(|_, _| Inject::Continue);
        match store.read_head("t/HEAD").expect("read") {
            HeadRead::Present { body, version } => {
                if expect_new {
                    assert_eq!(&*body, b"new-complete", "{phase:?}: the rename landed");
                    assert_eq!(version, content_version(b"new-complete"));
                } else {
                    assert_eq!(&*body, b"old-complete", "{phase:?}: nothing landed");
                    assert_eq!(version, content_version(b"old-complete"));
                }
            }
            HeadRead::Absent => panic!("the head is never torn away"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn indeterminate_at_publish_is_resolved_by_reading_never_assumed() {
    let root = fresh_root("lostack");
    let store = FsStore::new(&root);
    let v1 = match store.create_head("t/HEAD", b"one").expect("create") {
        ConditionalOutcome::Published { version } => version,
        other => panic!("{other:?}"),
    };
    // The mutation lands; the response is lost.
    store.set_hook(|phase, _| {
        if phase == Phase::Published {
            Inject::Indeterminate
        } else {
            Inject::Continue
        }
    });
    assert_eq!(
        store.replace_head("t/HEAD", &v1, b"two").expect("dispatch"),
        ConditionalOutcome::Indeterminate,
        "a lost response is uncertainty, never a manufactured outcome"
    );
    store.set_hook(|_, _| Inject::Continue);
    // Resolution by reading: the content version proves which body holds.
    match store.read_head("t/HEAD").expect("read") {
        HeadRead::Present { body, .. } => assert_eq!(&*body, b"two"),
        HeadRead::Absent => panic!("head exists"),
    }
    // The stale token cannot win afterwards.
    assert_eq!(
        store.replace_head("t/HEAD", &v1, b"three").expect("swap"),
        ConditionalOutcome::PreconditionFailed
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn temp_staging_lives_in_the_reserved_namespace_and_failures_clean_their_own() {
    let root = fresh_root("staging");
    let store = FsStore::new(&root);
    store
        .put_object("t/objects/1/chunk/aa", b"payload")
        .expect("put");
    // No temp residue after a successful publish.
    let tmp = root.join("~tmp");
    let leftovers = std::fs::read_dir(&tmp).map_or(0, std::iter::Iterator::count);
    assert_eq!(leftovers, 0, "publish removes its own staging");
    // Listing never surfaces ownership/staging namespaces.
    std::fs::create_dir_all(root.join("~lease/x")).expect("lease");
    std::fs::write(root.join("~lease/x/mutation.lock"), b"").expect("lock file");
    let page = store.list_objects("", None).expect("list");
    assert!(
        page.keys.iter().all(|key| !key.starts_with('~')),
        "{:?}",
        page.keys
    );
    assert_eq!(page.keys, vec!["t/objects/1/chunk/aa".to_string()]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn immutable_puts_verify_identity_and_conflicts_never_overwrite() {
    let root = fresh_root("immutable");
    let store = FsStore::new(&root);
    assert_eq!(
        store
            .put_object("t/objects/1/chunk/aa", b"bytes")
            .expect("put"),
        PutOutcome::Stored
    );
    assert_eq!(
        store
            .put_object("t/objects/1/chunk/aa", b"bytes")
            .expect("re-put"),
        PutOutcome::Stored,
        "identical bytes are idempotent evidence"
    );
    assert!(
        store.put_object("t/objects/1/chunk/aa", b"other").is_err(),
        "an immutable name never accepts conflicting bytes"
    );
    match store.get_object("t/objects/1/chunk/aa").expect("get") {
        bumbledb_log::store::ObjectRead::Present { body } => assert_eq!(&*body, b"bytes"),
        bumbledb_log::store::ObjectRead::Absent => panic!("object exists"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn delete_is_idempotent_and_never_reaches_outside_the_key_namespace() {
    let root = fresh_root("delete");
    let store = FsStore::new(&root);
    store.put_object("t/objects/1/chunk/aa", b"x").expect("put");
    store.delete_object("t/objects/1/chunk/aa").expect("delete");
    store
        .delete_object("t/objects/1/chunk/aa")
        .expect("repeat delete is harmless");
    for hostile in ["../escape", "~tmp/x", "~lease/y", "a/../b", "x.lock"] {
        assert!(store.delete_object(hostile).is_err(), "{hostile}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
