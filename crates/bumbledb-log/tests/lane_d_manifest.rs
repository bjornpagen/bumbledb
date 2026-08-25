//! The manifest and checkpoint documents: canonical render/parse
//! fixpoints, strict refusals, and the checkpoint order under CAS
//! contention.

mod lane_d_support;

use std::collections::BTreeMap;

use bumbledb_log::manifest::{
    Checkpoint, CheckpointError, Head, Manifest, ManifestError, Published, ckpt_json_key, hex32,
    log_key, manifest_key, publish_checkpoint,
};
use bumbledb_log::replica::Vector;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::{
    Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreKey, Swap,
};
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
    let bytes = manifest.render();
    assert_eq!(bytes.len(), 34);
    assert_eq!(bytes[0], 3);
    assert_eq!(&bytes[1..33], &digest(0x22));
    assert_eq!(bytes[33], 0);
}

#[test]
fn manifest_refuses_other_versions_by_name() {
    let mut bytes = Manifest {
        fingerprint: digest(0x33),
        checkpoint: None,
    }
    .render();
    bytes[0] = 2;
    assert_eq!(
        Manifest::parse(&bytes),
        Err(ManifestError::Version { got: 2 })
    );
}

#[test]
fn manifest_refuses_trailing_bytes() {
    let mut trailing = Manifest {
        fingerprint: digest(0x44),
        checkpoint: None,
    }
    .render();
    trailing.push(0);
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
        Vector::from(BTreeMap::from([
            (kitchen_braid(&codec), 4),
            (note_braid(&codec), 3),
        ]))
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
    const HEAD: usize = 52;
    let codec = codec();
    let doc = Checkpoint {
        braids: heads(1, 1),
        catalog: digest(0x72),
        writer: 1,
        prev: None,
    };
    let mut bytes = doc.render();
    let kitchen = kitchen_braid(&codec);
    let note = note_braid(&codec);
    assert!(kitchen < note, "BTreeMap order puts kitchen first");
    bytes[5 + HEAD..5 + HEAD + 4].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(
        Checkpoint::parse(&bytes, codec.braids()),
        Err(CheckpointError::UnknownBraid { got: 9 })
    );
}

#[test]
fn key_layout_matches_the_protocol() {
    let codec = codec();
    assert_eq!(manifest_key("").as_str(), "manifest");
    assert_eq!(manifest_key("t/acme").as_str(), "t/acme/manifest");
    assert_eq!(
        log_key("", kitchen_braid(&codec), 0x2a).as_str(),
        "log/c00000000/000000000000002a"
    );
    assert_eq!(
        ckpt_json_key("p", &digest(0x01)).as_str(),
        format!("p/ckpt/{}", hex32(&digest(0x01)))
    );
}

fn candidate(seed: u8, kitchen_g: u64, note_g: u64, prev: Option<[u8; 32]>) -> Checkpoint {
    Checkpoint {
        braids: heads(kitchen_g, note_g),
        catalog: digest(seed),
        writer: u64::from(seed),
        prev,
    }
}

fn publish(store: &FsStore, seed: u8, kitchen_g: u64, note_g: u64) -> ([u8; 32], Published) {
    let codec = codec();
    let doc = candidate(seed, kitchen_g, note_g, manifest_checkpoint(store));
    let id = doc.digest();
    let published = publish_checkpoint(store, "", codec.braids(), &doc).expect("publish");
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
/// its CAS attempt: the first swap lands `Moved`, and the caller
/// retries the same candidate document.
struct SwapInterloper {
    inner: FsStore,
    root: std::path::PathBuf,
    armed: std::sync::atomic::AtomicBool,
}

impl ObjectStore for SwapInterloper {
    fn get(&self, key: &StoreKey) -> StoreResult<Option<Fetched>> {
        self.inner.get(key)
    }

    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> StoreResult<Poll> {
        self.inner.get_if_changed(key, etag)
    }

    fn put_create<'a>(&self, key: &StoreKey, body: impl Into<Fenced<'a>>) -> StoreResult<Create> {
        self.inner.put_create(key, body)
    }

    fn put_swap<'a>(
        &self,
        key: &StoreKey,
        body: impl Into<Fenced<'a>>,
        etag: &Etag,
    ) -> StoreResult<Swap> {
        if key == &manifest_key("") && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst) {
            let plain = FsStore::new(self.root.clone());
            let codec = codec();
            let incumbent = Manifest::parse(
                &plain
                    .get(&manifest_key(""))
                    .expect("get")
                    .expect("manifest")
                    .bytes,
            )
            .expect("parses")
            .checkpoint;
            let published = publish_checkpoint(
                &plain,
                "",
                codec.braids(),
                &candidate(0xb1, 4, 4, incumbent),
            )
            .expect("interloper publishes");
            assert!(matches!(published, Published::Replaced));
        }
        self.inner.put_swap(key, body, etag)
    }

    fn delete(&self, key: &StoreKey) -> StoreResult<()> {
        self.inner.delete(key)
    }
}

#[test]
fn a_moved_cas_retries_the_same_document() {
    let root = temp_dir("ckpt_moved");
    let log = lane_d_support::TestLog::new(root.clone(), "");
    let plain = &log.store;

    let (first, _) = publish(plain, 0xc1, 1, 1);
    assert_eq!(manifest_checkpoint(plain), Some(first));

    // The caller bakes `first` into prev. The interloper lands before
    // the caller's CAS; Moved retries the same bytes, so the winner's
    // digest still names that baked prev and the interloper is orphan.
    let racing = SwapInterloper {
        inner: FsStore::new(root.clone()),
        root: root.clone(),
        armed: std::sync::atomic::AtomicBool::new(true),
    };
    let codec = codec();
    let winner_doc = candidate(0xc2, 9, 9, Some(first));
    let winner = winner_doc.digest();
    let published = publish_checkpoint(&racing, "", codec.braids(), &winner_doc)
        .expect("publish through the race");
    assert!(matches!(published, Published::Replaced));

    assert_eq!(manifest_checkpoint(plain), Some(winner));
    assert_eq!(doc_of(plain, winner).prev, Some(first));
    let interloper = candidate(0xb1, 4, 4, Some(first)).digest();
    assert_eq!(doc_of(plain, interloper).prev, Some(first));
    assert_ne!(manifest_checkpoint(plain), Some(interloper));
}

#[test]
fn checkpoint_order_race_converges_on_the_greater_sum() {
    let root = temp_dir("ckpt_race");
    let log = lane_d_support::TestLog::new(root.clone(), "");

    let outcome_b = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            let store = FsStore::new(root.clone());
            let codec = codec();
            publish_checkpoint(&store, "", codec.braids(), &candidate(0x91, 2, 1, None))
                .expect("small racer")
        });
        let b = scope.spawn(|| {
            let store = FsStore::new(root.clone());
            let codec = codec();
            publish_checkpoint(&store, "", codec.braids(), &candidate(0x92, 8, 8, None))
                .expect("big racer")
        });
        let _ = a.join().expect("small thread");
        b.join().expect("big thread")
    });
    assert!(matches!(outcome_b, Published::Replaced));
    assert_eq!(
        manifest_checkpoint(&log.store),
        Some(candidate(0x92, 8, 8, None).digest())
    );
}
