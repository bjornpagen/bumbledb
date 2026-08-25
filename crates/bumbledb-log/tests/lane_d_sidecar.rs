//! The chain sidecar: canonical fixpoints, strict refusals, and the
//! atomic write discipline.

mod lane_d_support;

use bumbledb_log::sidecar::{CHAIN_FILE, Chain, ChainEntry, Pending, SidecarError};
use lane_d_support::{codec, kitchen_braid, note_braid, temp_dir};

#[test]
fn genesis_materializes_every_braid() {
    let codec = codec();
    let chain = Chain::genesis(codec.braids());
    assert_eq!(chain.entries.len(), 2);
    assert_eq!(chain.sum(), 0);
    assert_eq!(chain.position(kitchen_braid(&codec)), ChainEntry::GENESIS);
    assert!(chain.pending.is_none());
}

#[test]
fn render_parse_fixpoint_with_and_without_pending() {
    let codec = codec();
    let mut chain = Chain::genesis(codec.braids());
    chain.entries.insert(
        kitchen_braid(&codec),
        ChainEntry {
            g: 80,
            prev: [0x5a; 32],
            ts: 1_755_801_600_000,
        },
    );
    let bytes = chain.render();
    assert_eq!(Chain::parse(&bytes, codec.braids()), Ok(chain.clone()));

    chain.pending = Some(Pending {
        braid: note_braid(&codec),
        slot: 81,
        bytes: vec![0xde, 0xad, 0xbe, 0xef],
    });
    let bytes = chain.render();
    assert_eq!(Chain::parse(&bytes, codec.braids()), Ok(chain));
}

#[test]
fn parse_refuses_version_whitespace_and_unknown_braids() {
    let codec = codec();
    let canonical = String::from_utf8(Chain::genesis(codec.braids()).render()).expect("utf8");

    let versioned = canonical.replace("{\"v\":3,", "{\"v\":2,");
    assert_eq!(
        Chain::parse(versioned.as_bytes(), codec.braids()),
        Err(SidecarError::Version { got: 2 })
    );

    let spaced = canonical.replace(',', ", ");
    assert!(matches!(
        Chain::parse(spaced.as_bytes(), codec.braids()),
        Err(SidecarError::Malformed { .. })
    ));

    let foreign = canonical.replace("\"c00000002\"", "\"c00000007\"");
    assert_eq!(
        Chain::parse(foreign.as_bytes(), codec.braids()),
        Err(SidecarError::UnknownBraid { got: 7 })
    );
}

#[test]
fn parse_is_order_strict_like_the_checkpoint_parser() {
    let codec = codec();
    let mut chain = Chain::genesis(codec.braids());
    chain.entries.insert(
        kitchen_braid(&codec),
        ChainEntry {
            g: 3,
            prev: [0x11; 32],
            ts: 100,
        },
    );
    chain.entries.insert(
        note_braid(&codec),
        ChainEntry {
            g: 5,
            prev: [0x22; 32],
            ts: 200,
        },
    );
    let canonical = String::from_utf8(chain.render()).expect("utf8");
    let kitchen = format!("\"{}\"", kitchen_braid(&codec));
    let note = format!("\"{}\"", note_braid(&codec));
    let kitchen_entry_start = canonical.find(&kitchen).expect("kitchen entry");
    let note_entry_start = canonical.find(&note).expect("note entry");
    assert!(kitchen_entry_start < note_entry_start, "canonical order");

    // The same two facts in swapped order are non-canonical bytes of
    // the same value; the order-strict walk refuses them, so an
    // accepted sidecar always re-renders byte-identically.
    let kitchen_body = &canonical[kitchen_entry_start
        ..canonical[kitchen_entry_start..]
            .find("},")
            .map(|end| kitchen_entry_start + end + 1)
            .expect("kitchen body end")];
    let note_body = &canonical[note_entry_start
        ..canonical[note_entry_start..]
            .find('}')
            .map(|end| note_entry_start + end + 1)
            .expect("note body end")];
    let swapped = canonical.replacen(
        &format!("{kitchen_body},{note_body}"),
        &format!("{note_body},{kitchen_body}"),
        1,
    );
    assert_ne!(swapped, canonical, "the swap changed the bytes");
    assert!(matches!(
        Chain::parse(swapped.as_bytes(), codec.braids()),
        Err(SidecarError::Malformed { .. })
    ));

    let duplicated = canonical.replacen(
        &format!("{kitchen_body},{note_body}"),
        &format!("{kitchen_body},{kitchen_body}"),
        1,
    );
    assert!(matches!(
        Chain::parse(duplicated.as_bytes(), codec.braids()),
        Err(SidecarError::Malformed { .. })
    ));
}

#[test]
fn vector_and_sum_agree_with_entries() {
    let codec = codec();
    let mut chain = Chain::genesis(codec.braids());
    chain
        .entries
        .get_mut(&kitchen_braid(&codec))
        .expect("kitchen entry")
        .g = 5;
    chain
        .entries
        .get_mut(&note_braid(&codec))
        .expect("note entry")
        .g = 2;
    assert_eq!(chain.sum(), 7);
    assert_eq!(chain.vector()[&kitchen_braid(&codec)], 5);
    assert_eq!(chain.vector()[&note_braid(&codec)], 2);
}

#[test]
fn write_atomic_then_read_roundtrips_and_replaces() {
    let codec = codec();
    let dir = temp_dir("sidecar_atomic");
    let mut chain = Chain::genesis(codec.braids());
    chain.write_atomic(&dir).expect("first write");
    assert_eq!(
        Chain::read(&dir, codec.braids()).expect("read"),
        Some(Ok(chain.clone()))
    );

    chain
        .entries
        .get_mut(&kitchen_braid(&codec))
        .expect("kitchen entry")
        .g = 9;
    chain.write_atomic(&dir).expect("second write");
    assert_eq!(
        Chain::read(&dir, codec.braids()).expect("read"),
        Some(Ok(chain))
    );

    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .expect("read dir")
        .map(|entry| entry.expect("entry").file_name())
        .filter(|name| name.to_string_lossy() != CHAIN_FILE)
        .collect();
    assert!(leftovers.is_empty(), "no temp files survive: {leftovers:?}");
}

#[test]
fn read_of_a_missing_sidecar_is_none() {
    let codec = codec();
    let dir = temp_dir("sidecar_missing");
    assert_eq!(Chain::read(&dir, codec.braids()).expect("read"), None);
}
