//! The manifest and checkpoint documents: canonical render/parse
//! fixpoints, strict refusals, and the checkpoint order under CAS
//! contention.

mod lane_d_support;

use std::collections::BTreeMap;

use bumbledb_log::manifest::{
    Checkpoint, CheckpointError, Head, Manifest, ManifestError, Published, ckpt_json_key, log_key,
    manifest_key, publish_checkpoint,
};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
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

fn upload_checkpoint(store: &FsStore, seed: u8, kitchen_g: u64, note_g: u64) -> ([u8; 32], u64) {
    let doc = Checkpoint {
        braids: heads(kitchen_g, note_g),
        catalog: digest(seed),
        writer: u64::from(seed),
        prev: None,
    };
    let id = digest(seed);
    store
        .put_create(&ckpt_json_key("", &id), &doc.render())
        .expect("upload doc");
    (id, doc.sum())
}

#[test]
fn checkpoint_order_keeps_the_greater_sum() {
    let root = temp_dir("ckpt_order");
    let log = lane_d_support::TestLog::new(root, "");
    let store = &log.store;
    let braids = log.codec.braids();

    let (big, big_sum) = upload_checkpoint(store, 0x81, 6, 4);
    let (small, small_sum) = upload_checkpoint(store, 0x82, 3, 2);

    assert!(matches!(
        publish_checkpoint(store, "", braids, big, big_sum).expect("publish big"),
        Published::Replaced
    ));
    match publish_checkpoint(store, "", braids, small, small_sum).expect("publish small") {
        Published::Kept { incumbent } => assert_eq!(incumbent, big),
        other => panic!("small candidate must lose, got {other:?}"),
    }
    let manifest = Manifest::parse(
        &store
            .get(&manifest_key(""))
            .expect("get")
            .expect("manifest")
            .bytes,
    )
    .expect("parses");
    assert_eq!(manifest.checkpoint, Some(big));
}

#[test]
fn checkpoint_order_race_converges_on_the_greater_sum() {
    let root = temp_dir("ckpt_race");
    let log = lane_d_support::TestLog::new(root.clone(), "");
    let braids = log.codec.braids();

    let (small, small_sum) = upload_checkpoint(&log.store, 0x91, 2, 1);
    let (big, big_sum) = upload_checkpoint(&log.store, 0x92, 8, 8);

    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            let store = FsStore::new(root.clone());
            let codec = codec();
            publish_checkpoint(&store, "", codec.braids(), small, small_sum).expect("small racer")
        });
        let b = scope.spawn(|| {
            let store = FsStore::new(root.clone());
            let codec = codec();
            publish_checkpoint(&store, "", codec.braids(), big, big_sum).expect("big racer")
        });
        let _ = a.join().expect("small thread");
        let outcome_b = b.join().expect("big thread");
        assert!(matches!(outcome_b, Published::Replaced));
    });

    let manifest = Manifest::parse(
        &log.store
            .get(&manifest_key(""))
            .expect("get")
            .expect("manifest")
            .bytes,
    )
    .expect("parses");
    assert_eq!(manifest.checkpoint, Some(big));
    let _ = braids;
}
