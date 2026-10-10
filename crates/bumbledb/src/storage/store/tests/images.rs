//! Compacted images: a copy of the committed state that installs as a
//! database with the same identity, head, host records and content.

use super::*;
use crate::storage::store::host::Head;

#[test]
#[cfg_attr(miri, ignore)]
fn an_image_installs_with_the_same_identity_head_and_content() {
    let (dir, path) = store_dir("image-source");
    let store = create_default(&path);
    for id in 0..64 {
        commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(id, &"x".repeat(200)))], &[]),
        );
    }
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &[],
            &(0..32)
                .map(|id| (NOTE, note(id, &"x".repeat(200))))
                .collect::<Vec<_>>(),
        ),
    );
    {
        let context = work();
        let mut owner = store.writer(&context).expect("writer");
        let records = host_put(b"r/1", b"receipt");
        owner
            .prepare_unchanged()
            .expect("txn")
            .seal(HostChanges {
                records: &records,
                head: Head::Put(b"seq 66"),
            })
            .expect("seal")
            .commit()
            .expect("commit");
    }
    let image = dir.path().join("image.mdb");
    store.write_image(&image, &work()).expect("image");
    assert!(
        std::fs::metadata(&image).expect("image").len()
            <= store.file_bytes().expect("source bytes")
    );
    let dest = dir.path().join("installed");
    let installed =
        Store::install_image(&image, &dest, &schema(), Options::default()).expect("install");
    assert!(!image.exists(), "the image file was moved into place");
    assert_eq!(installed.identity().database, store.identity().database);
    let source = store.snapshot(&work()).expect("source");
    let copy = installed.snapshot(&work()).expect("copy");
    assert_eq!(copy.generation(), source.generation());
    assert_eq!(copy.head().expect("head"), Some(b"seq 66".as_slice()));
    assert_eq!(
        copy.host_record(b"r/1").expect("record"),
        Some(b"receipt".as_slice())
    );
    assert_eq!(
        copy.content_digest(&work()).expect("copy digest"),
        source.content_digest(&work()).expect("source digest")
    );
    assert_eq!(copy.row_count(NOTE).expect("count"), 32);
}

#[test]
#[cfg_attr(miri, ignore)]
fn an_image_of_another_schema_or_format_refuses_and_leaves_no_destination() {
    let (dir, path) = store_dir("image-refusal");
    let store = create_default(&path);
    let image = dir.path().join("image.mdb");
    store.write_image(&image, &work()).expect("image");
    let dest = dir.path().join("installed");
    assert!(matches!(
        Store::install_image(&image, &dest, &other_schema(), Options::default()),
        Err(Error::SchemaMismatch)
    ));
    assert!(!dest.exists());
    let garbage = dir.path().join("garbage.mdb");
    std::fs::write(&garbage, vec![0u8; 8192]).expect("garbage");
    assert!(Store::install_image(&garbage, &dest, &schema(), Options::default()).is_err());
    assert!(!dest.exists());
}
