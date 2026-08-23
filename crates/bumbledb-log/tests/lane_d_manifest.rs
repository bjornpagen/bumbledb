//! The manifest and checkpoint documents: canonical render/parse
//! fixpoints, strict refusals, and the checkpoint order under CAS
//! contention.

mod lane_d_support;

use std::collections::BTreeMap;

use bumbledb_log::manifest::{
    Checkpoint, CheckpointError, Head, Manifest, ManifestError, Published, ckpt_json_key, log_key,
    manifest_key, publish_checkpoint,
};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{Create, Etag, Fetched, ObjectStore, Poll, Result as StoreResult, Swap};
use lane_d_support::{codec, kitchen_braid, note_braid, temp_dir};

fn digest(seed: u8) -> [u8; 32] {
    [seed; 32]
}

#[test]
fn manifest_render_parse_fixpoint() {
    for checkpoint in [None, Some(digest(0xab))] {
        let manifest = Manifest {
            fingerprint: digest(0x11),
            checkpoint,
        };
        let bytes = manifest.render();
        assert_eq!(Manifest::parse(&bytes), Ok(manifest));
    }
}

#[test]
fn manifest_null_checkpoint_is_a_real_null() {
    let manifest = Manifest {
        fingerprint: digest(0x22),
        checkpoint: None,
    };
    let text = String::from_utf8(manifest.render()).expect("utf8");
    assert!(text.ends_with("\"checkpoint\":null}"));
    assert!(!text.contains("\"\""));
}

#[test]
fn manifest_refuses_other_versions_by_name() {
    let manifest = Manifest {
        fingerprint: digest(0x33),
        checkpoint: None,
    };
    let bytes = String::from_utf8(manifest.render()).expect("utf8");
    let hostile = bytes.replace("{\"v\":2,", "{\"v\":3,");
    assert_eq!(
        Manifest::parse(hostile.as_bytes()),
        Err(ManifestError::Version { got: 3 })
    );
}

#[test]
fn manifest_refuses_whitespace_reordering_and_trailing_bytes() {
    let canonical = String::from_utf8(
        Manifest {
            fingerprint: digest(0x44),
            checkpoint: None,
        }
        .render(),
    )
    .expect("utf8");
    let spaced = canonical.replace(':', ": ");
    assert!(matches!(
        Manifest::parse(spaced.as_bytes()),
        Err(ManifestError::Malformed { .. })
    ));
    let mut trailing = canonical.clone().into_bytes();
    trailing.push(b'\n');
    assert!(matches!(
        Manifest::parse(&trailing),
        Err(ManifestError::Malformed { .. })
    ));
}

fn heads(kitchen_g: u64, note_g: u64) -> BTreeMap<bumbledb_log::braids::BraidId, Head> {
    let codec = codec();
    BTreeMap::from([
        (
            kitchen_braid(&codec),
            Head {
                g: kitchen_g,
                hash: digest(0x51),
                ts: 1_000,
            },
        ),
        (
            note_braid(&codec),
            Head {
                g: note_g,
                hash: digest(0x52),
                ts: 2_000,
            },
        ),
    ])
}

#[test]
fn checkpoint_render_parse_fixpoint_and_sum() {
    let codec = codec();
    let doc = Checkpoint {
        braids: heads(4, 3),
        catalog: digest(0x61),
        writer: 12,
        prev: Some(digest(0x62)),
    };
    let bytes = doc.render();
    let parsed = Checkpoint::parse(&bytes, codec.braids()).expect("parses");
    assert_eq!(parsed, doc);
    assert_eq!(parsed.sum(), 7);
    assert_eq!(
        parsed.vector(),
        BTreeMap::from([(kitchen_braid(&codec), 4), (note_braid(&codec), 3)])
    );
}

#[test]
fn checkpoint_refuses_missing_braid_as_set_drift() {
    let codec = codec();
    let mut doc = Checkpoint {
        braids: heads(1, 1),
        catalog: digest(0x71),
        writer: 1,
        prev: None,
    };
    doc.braids.remove(&note_braid(&codec));
    let bytes = doc.render();
    assert_eq!(
        Checkpoint::parse(&bytes, codec.braids()),
        Err(CheckpointError::BraidSet)
    );
}

#[test]
fn checkpoint_refuses_a_braid_the_schema_never_minted() {
    let codec = codec();
    let doc = Checkpoint {
        braids: heads(1, 1),
        catalog: digest(0x72),
        writer: 1,
        prev: None,
    };
    let text = String::from_utf8(doc.render()).expect("utf8");
    let hostile = text.replace("\"c00000002\"", "\"c00000009\"");
    assert_eq!(
        Checkpoint::parse(hostile.as_bytes(), codec.braids()),
        Err(CheckpointError::UnknownBraid { got: 9 })
    );
}

#[test]
fn key_layout_matches_the_protocol() {
    let codec = codec();
    assert_eq!(manifest_key(""), "manifest.json");
    assert_eq!(manifest_key("t/acme"), "t/acme/manifest.json");
    assert_eq!(
        log_key("", kitchen_braid(&codec), 0x2a),
        "log/c00000000/000000000000002a"
    );
    assert!(ckpt_json_key("p", &digest(0x01)).starts_with("p/ckpt/"));
    assert_eq!(
        ckpt_json_key("p", &digest(0x01)).split('.').next_back(),
        Some("json")
    );
}

fn publish(store: &FsStore, seed: u8, kitchen_g: u64, note_g: u64) -> ([u8; 32], Published) {
    let codec = codec();
    let id = digest(seed);
    let published = publish_checkpoint(
        store,
        "",
        codec.braids(),
        id,
        &heads(kitchen_g, note_g),
        digest(seed),
        u64::from(seed),
    )
    .expect("publish");
    (id, published)
}

fn manifest_checkpoint(store: &FsStore) -> Option<[u8; 32]> {
    Manifest::parse(
        &store
            .get(&manifest_key(""))
            .expect("get")
            .expect("manifest")
            .bytes,
    )
    .expect("parses")
    .checkpoint
}

fn doc_of(store: &FsStore, id: [u8; 32]) -> Checkpoint {
    let codec = codec();
    Checkpoint::parse(
        &store
            .get(&ckpt_json_key("", &id))
            .expect("get")
            .expect("doc")
            .bytes,
        codec.braids(),
    )
    .expect("doc parses")
}

#[test]
fn checkpoint_order_keeps_the_greater_sum() {
    let root = temp_dir("ckpt_order");
    let log = lane_d_support::TestLog::new(root, "");
    let store = &log.store;

    let (big, published_big) = publish(store, 0x81, 6, 4);
    assert!(matches!(published_big, Published::Replaced));
    let (small, published_small) = publish(store, 0x82, 3, 2);
    match published_small {
        Published::Kept { incumbent } => assert_eq!(incumbent, big),
        other => panic!("small candidate must lose, got {other:?}"),
    }
    assert_eq!(manifest_checkpoint(store), Some(big));
    let _ = small;
}

#[test]
fn every_incumbent_stays_reachable_by_the_backlink_walk() {
    let root = temp_dir("ckpt_backlink");
    let log = lane_d_support::TestLog::new(root, "");
    let store = &log.store;

    let (first, _) = publish(store, 0xa1, 1, 1);
    let (second, _) = publish(store, 0xa2, 2, 2);
    let (third, _) = publish(store, 0xa3, 3, 3);

    assert_eq!(manifest_checkpoint(store), Some(third));
    assert_eq!(
        doc_of(store, third).prev,
        Some(second),
        "prev names the incumbent the CAS actually replaced"
    );
    assert_eq!(doc_of(store, second).prev, Some(first));
    assert_eq!(doc_of(store, first).prev, None);
}

/// Injects a competing publication between a caller's manifest read and
/// its CAS attempt: the first swap lands `Moved`, forcing the caller
/// through the rebuild-prev arm.
struct SwapInterloper {
    inner: FsStore,
    root: std::path::PathBuf,
    armed: std::sync::atomic::AtomicBool,
}

impl ObjectStore for SwapInterloper {
    fn get(&self, key: &str) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &str, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create(&self, key: &str, bytes: &[u8]) -> StoreResult<Create> {
        self.inner.put_create(key, bytes)
    }

    fn put_swap(&self, key: &str, bytes: &[u8], etag: &Etag) -> StoreResult<Swap> {
        if key == manifest_key("") && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst) {
            let plain = FsStore::new(self.root.clone());
            let codec = codec();
            let published = publish_checkpoint(
                &plain,
                "",
                codec.braids(),
                digest(0xb1),
                &heads(4, 4),
                digest(0xb1),
                0xb1,
            )
            .expect("interloper publishes");
            assert!(matches!(published, Published::Replaced));
        }
        self.inner.put_swap(key, bytes, etag)
    }

    fn delete(&self, key: &str) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

#[test]
fn a_moved_cas_rebuilds_prev_to_the_incumbent_actually_replaced() {
    let root = temp_dir("ckpt_moved");
    let log = lane_d_support::TestLog::new(root.clone(), "");
    let plain = &log.store;

    let (first, _) = publish(plain, 0xc1, 1, 1);
    assert_eq!(manifest_checkpoint(plain), Some(first));

    // The caller reads incumbent `first`, but the interloper lands a
    // greater checkpoint before the caller's CAS: the caller's first
    // swap is Moved, and the rebuild must re-point prev at the
    // interloper — the incumbent the winning swap actually replaces.
    let racing = SwapInterloper {
        inner: FsStore::new(root.clone()),
        root: root.clone(),
        armed: std::sync::atomic::AtomicBool::new(true),
    };
    let codec = codec();
    let winner = digest(0xc2);
    let published = publish_checkpoint(
        &racing,
        "",
        codec.braids(),
        winner,
        &heads(9, 9),
        digest(0xc2),
        0xc2,
    )
    .expect("publish through the race");
    assert!(matches!(published, Published::Replaced));

    assert_eq!(manifest_checkpoint(plain), Some(winner));
    assert_eq!(
        doc_of(plain, winner).prev,
        Some(digest(0xb1)),
        "prev is proven by the CAS: it names the interloper, never the stale read"
    );
    assert_eq!(doc_of(plain, digest(0xb1)).prev, Some(first));
}

#[test]
fn checkpoint_order_race_converges_on_the_greater_sum() {
    let root = temp_dir("ckpt_race");
    let log = lane_d_support::TestLog::new(root.clone(), "");

    let outcome_b = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            let store = FsStore::new(root.clone());
            let codec = codec();
            publish_checkpoint(
                &store,
                "",
                codec.braids(),
                digest(0x91),
                &heads(2, 1),
                digest(0x91),
                0x91,
            )
            .expect("small racer")
        });
        let b = scope.spawn(|| {
            let store = FsStore::new(root.clone());
            let codec = codec();
            publish_checkpoint(
                &store,
                "",
                codec.braids(),
                digest(0x92),
                &heads(8, 8),
                digest(0x92),
                0x92,
            )
            .expect("big racer")
        });
        let _ = a.join().expect("small thread");
        b.join().expect("big thread")
    });
    assert!(matches!(outcome_b, Published::Replaced));
    assert_eq!(manifest_checkpoint(&log.store), Some(digest(0x92)));
}
