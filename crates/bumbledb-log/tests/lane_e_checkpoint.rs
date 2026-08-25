//! Checkpoint duty: cadence crossings publish `ckpt/{digest}.mdb` and
//! `ckpt/{digest}` and run the manifest CAS off the commit loop;
//! races resolve by the checkpoint order (greater sum replaces,
//! otherwise the incumbent stays and the loser is a known orphan).

mod lane_e_support;

use std::collections::BTreeMap;

use bumbledb::SchemaDescriptor;
use bumbledb_log::manifest::{
    Checkpoint, Head, Manifest, ckpt_json_key, ckpt_mdb_key, manifest_key, publish_checkpoint,
};
use bumbledb_log::replica::{Opened, Provenance, Replica};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Options, Writer, WriterOpened};
use lane_e_support::{NOTE, codec, note_braid, note_row, temp_dir, theory};

fn open_at(root: std::path::PathBuf, dir: &std::path::Path) -> Writer<SchemaDescriptor, FsStore> {
    match Writer::open(FsStore::new(root), "", dir, theory(), Options::new(81))
        .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn crossing_the_sum_cadence_publishes_a_checkpoint() {
    let root = temp_dir("duty");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir);
    writer.set_checkpoint_cadence(2, u64::MAX);

    for id in 0..2u64 {
        assert!(matches!(
            writer
                .commit(|batch| {
                    batch.insert(NOTE, [note_row(id, "cadence")]);
                    Ok(())
                })
                .expect("commit"),
            Commit::Accepted { .. }
        ));
    }
    writer.quiesce();

    let store = FsStore::new(root.clone());
    let manifest = Manifest::parse(
        &store
            .get(&manifest_key(""))
            .expect("get")
            .expect("manifest")
            .bytes,
    )
    .expect("manifest parses");
    let digest = manifest.checkpoint.expect("checkpoint published");

    let codec = codec();
    let doc_bytes = store
        .get(&ckpt_json_key("", &digest))
        .expect("get")
        .expect("checkpoint doc");
    let doc = Checkpoint::parse(&doc_bytes.bytes, codec.braids()).expect("doc parses");
    assert_eq!(doc.writer, 81);
    assert_eq!(doc.prev, None);
    assert_eq!(doc.braids[&note_braid(&codec)].g, 2);
    assert_eq!(doc.sum(), 2);

    let mdb = store
        .get(&ckpt_mdb_key("", &digest))
        .expect("get")
        .expect("checkpoint object");
    assert_eq!(
        *blake3::hash(&mdb.bytes).as_bytes(),
        digest,
        "content-addressed: the name is the digest of the bytes"
    );
    drop(writer);

    // The checkpoint the writer published seeds a fresh replica — the
    // cross-consumer proof that the duty's shape is lane D's shape.
    let seeded = Replica::open(
        FsStore::new(root.clone()),
        "",
        &root.join("replica"),
        theory(),
    )
    .expect("replica open");
    let Opened::Ready(replica) = seeded else {
        panic!("replica seeds from the writer's checkpoint");
    };
    assert_eq!(replica.provenance(), Provenance::Checkpoint);
    assert_eq!(replica.vector()[&note_braid(&codec)], 2);
}

#[test]
fn crossing_the_byte_cadence_publishes_too() {
    let root = temp_dir("duty_bytes");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir);
    writer.set_checkpoint_cadence(u64::MAX, 1);

    writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "bytes")]);
            Ok(())
        })
        .expect("commit");
    writer.quiesce();
    let store = FsStore::new(root);
    let manifest = Manifest::parse(
        &store
            .get(&manifest_key(""))
            .expect("get")
            .expect("manifest")
            .bytes,
    )
    .expect("manifest parses");
    assert!(manifest.checkpoint.is_some());
}

#[test]
fn the_checkpoint_order_keeps_a_greater_incumbent() {
    let root = temp_dir("order");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir);

    writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "first")]);
            Ok(())
        })
        .expect("commit");

    // Plant an incumbent whose vector sum outranks anything the writer
    // can produce here; the writer's duty must lose the CAS race and
    // leave the incumbent standing.
    let codec = codec();
    let store = FsStore::new(root.clone());
    let heads: BTreeMap<_, _> = codec
        .braids()
        .components()
        .keys()
        .map(|braid| {
            (
                *braid,
                Head {
                    g: if *braid == note_braid(&codec) { 50 } else { 0 },
                    hash: [7u8; 32],
                    ts: 1,
                },
            )
        })
        .collect();
    let prev = store
        .get(&manifest_key(""))
        .expect("get")
        .and_then(|fetched| Manifest::parse(&fetched.bytes).ok()?.checkpoint);
    let incumbent = Checkpoint {
        braids: heads,
        catalog: [9u8; 32],
        writer: 999,
        prev,
    };
    let incumbent_digest = incumbent.digest();
    assert!(matches!(
        publish_checkpoint(&store, "", codec.braids(), &incumbent).expect("publish"),
        bumbledb_log::manifest::Published::Replaced
    ));

    writer.set_checkpoint_cadence(1, u64::MAX);
    writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2, "second")]);
            Ok(())
        })
        .expect("commit");
    writer.quiesce();

    let manifest = Manifest::parse(
        &store
            .get(&manifest_key(""))
            .expect("get")
            .expect("manifest")
            .bytes,
    )
    .expect("manifest parses");
    assert_eq!(
        manifest.checkpoint,
        Some(incumbent_digest),
        "the incumbent's sum is at least the candidate's; the loser is a known orphan"
    );
}
