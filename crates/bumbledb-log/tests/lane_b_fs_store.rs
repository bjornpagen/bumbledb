//! On-disk protocol of `FsStore`: parent directories, the dead-owner
//! lockfile, and the computed etag that is never stored. Five-verb
//! semantics that do not touch a disk live on `MemStore`.

use std::path::PathBuf;

use bumbledb_log::store::fs::{FsStore, content_etag};
use bumbledb_log::store::{Create, ObjectStore, StoreKey, Swap};

fn fresh_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "bdb-log-b-store-{}-{name}-{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create test root");
    root
}

#[test]
fn put_create_makes_parent_directories() {
    let store = FsStore::new(fresh_root("create_parents"));
    let key = StoreKey::of("ckpt/deep/nested/prefix/object.json");
    assert!(matches!(
        store.put_create(&key, b"{}").expect("create"),
        Create::Created(_)
    ));
    assert!(store.get(&key).expect("get").is_some());
}

#[test]
fn malformed_keys_are_refused_at_the_boundary() {
    for key in [
        "",
        "/abs",
        "a//b",
        "../escape",
        "a/./b",
        "trailing/",
        "manifest.lock",
        "a.lock/b",
    ] {
        assert!(StoreKey::parse(key).is_err(), "key {key:?} must be refused");
    }
}

#[test]
fn a_dead_owners_lockfile_beside_the_key_is_broken_by_put_swap() {
    let root = fresh_root("dead_lock");
    let store = FsStore::new(&root);
    let Create::Created(birth) = store
        .put_create(&StoreKey::of("manifest"), b"v1")
        .expect("create")
    else {
        panic!("fresh key must be Created");
    };
    std::fs::write(root.join("manifest.lock"), b"999999999").expect("plant dead lock");
    let swapped = store
        .put_swap(&StoreKey::of("manifest"), b"v2", &birth)
        .expect("swap");
    assert_eq!(swapped, Swap::Swapped(content_etag(b"v2")));
    assert!(
        !root.join("manifest.lock").exists(),
        "the broken lock leaves no residue"
    );
}

#[test]
fn the_verbs_leave_no_sidecar_beside_the_object() {
    let root = fresh_root("no_sidecar");
    let store = FsStore::new(&root);
    let Create::Created(birth) = store
        .put_create(&StoreKey::of("manifest"), b"v1")
        .expect("create")
    else {
        panic!("fresh key must be Created");
    };
    store
        .put_swap(&StoreKey::of("manifest"), b"v2", &birth)
        .expect("swap");
    store.get(&StoreKey::of("manifest")).expect("get");
    let names: Vec<String> = std::fs::read_dir(&root)
        .expect("read root")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        names,
        vec!["manifest".to_string()],
        "the object's bytes are the whole on-disk record"
    );
}
