//! The chain sidecar: canonical fixpoints, strict refusals, and the
//! atomic write discipline.

mod lane_d_support;

use bumbledb_log::sidecar::{CHAIN_FILE, Chain, ChainEntry, Pending, SidecarError, SidecarRead};
use lane_d_support::{codec, kitchen_braid, note_braid, temp_dir};

#[test]
fn genesis_materializes_every_braid() {
    let codec = codec();
    let chain = Chain::genesis(codec.braids());
    assert_eq!(chain.entries().len(), 2);
    assert_eq!(chain.sum(), 0);
    assert_eq!(chain.position(kitchen_braid(&codec)), ChainEntry::GENESIS);
    assert!(matches!(chain, Chain::Settled { .. }));
}

#[test]
fn render_parse_fixpoint_with_and_without_pending() {
    let codec = codec();
    let mut chain = Chain::genesis(codec.braids());
    chain.entries_mut().insert(
        kitchen_braid(&codec),
        ChainEntry {
            g: 80,
            prev: [0x5a; 32],
            ts: 1_755_801_600_000,
        },
    );
    let bytes = chain.render();
    assert_eq!(Chain::parse(&bytes, codec.braids()), Ok(chain.clone()));

    let Chain::Settled { entries } = chain else {
        panic!("genesis is Settled");
    };
    let chain = Chain::Pending {
        entries,
        batch: Pending {
            braid: note_braid(&codec),
            slot: 81,
            bytes: vec![0xde, 0xad, 0xbe, 0xef],
        },
    };
    let bytes = chain.render();
    assert_eq!(Chain::parse(&bytes, codec.braids()), Ok(chain));
}

#[test]
fn parse_refuses_version_trailing_bytes_and_unknown_braids() {
    let codec = codec();
    let canonical = Chain::genesis(codec.braids()).render();

    let mut versioned = canonical.clone();
    versioned[0] = 2;
    assert_eq!(
        Chain::parse(&versioned, codec.braids()),
        Err(SidecarError::Version { got: 2 })
    );

    let mut trailing = canonical.clone();
    trailing.push(0);
    assert!(matches!(
        Chain::parse(&trailing, codec.braids()),
        Err(SidecarError::Malformed { .. })
    ));

    let mut foreign = canonical;
    foreign[5..9].copy_from_slice(&7u32.to_le_bytes());
    assert_eq!(
        Chain::parse(&foreign, codec.braids()),
        Err(SidecarError::UnknownBraid { got: 7 })
    );
}

#[test]
fn parse_is_order_strict_like_the_checkpoint_parser() {
    const ENTRY: usize = 52;
    let codec = codec();
    let mut chain = Chain::genesis(codec.braids());
    chain.entries_mut().insert(
        kitchen_braid(&codec),
        ChainEntry {
            g: 3,
            prev: [0x11; 32],
            ts: 100,
        },
    );
    chain.entries_mut().insert(
        note_braid(&codec),
        ChainEntry {
            g: 5,
            prev: [0x22; 32],
            ts: 200,
        },
    );
    let canonical = chain.render();
    assert!(
        kitchen_braid(&codec) < note_braid(&codec),
        "canonical order"
    );
    let kitchen = canonical[5..5 + ENTRY].to_vec();
    let note = canonical[5 + ENTRY..5 + 2 * ENTRY].to_vec();

    // The same two facts in swapped order are non-canonical bytes of
    // the same value; the order-strict walk refuses them, so an
    // accepted sidecar always re-renders byte-identically.
    let mut swapped = canonical.clone();
    swapped[5..5 + ENTRY].copy_from_slice(&note);
    swapped[5 + ENTRY..5 + 2 * ENTRY].copy_from_slice(&kitchen);
    assert_ne!(swapped, canonical, "the swap changed the bytes");
    assert!(matches!(
        Chain::parse(&swapped, codec.braids()),
        Err(SidecarError::Malformed { .. })
    ));

    let mut duplicated = canonical;
    duplicated[5 + ENTRY..5 + 2 * ENTRY].copy_from_slice(&kitchen);
    assert!(matches!(
        Chain::parse(&duplicated, codec.braids()),
        Err(SidecarError::Malformed { .. })
    ));
}

#[test]
fn vector_and_sum_agree_with_entries() {
    let codec = codec();
    let mut chain = Chain::genesis(codec.braids());
    chain
        .entries_mut()
        .get_mut(&kitchen_braid(&codec))
        .expect("kitchen entry")
        .g = 5;
    chain
        .entries_mut()
        .get_mut(&note_braid(&codec))
        .expect("note entry")
        .g = 2;
    assert_eq!(chain.sum(), 7);
    assert_eq!(chain.vector().at(kitchen_braid(&codec)), 5);
    assert_eq!(chain.vector().at(note_braid(&codec)), 2);
}

#[test]
fn write_atomic_then_read_roundtrips_and_replaces() {
    let codec = codec();
    let dir = temp_dir("sidecar_atomic");
    let mut chain = Chain::genesis(codec.braids());
    chain.write_atomic(&dir).expect("first write");
    match Chain::read(&dir, codec.braids()) {
        SidecarRead::Read(got) => assert_eq!(got, chain),
        other => panic!("expected Read, got {}", other.identity()),
    }

    chain
        .entries_mut()
        .get_mut(&kitchen_braid(&codec))
        .expect("kitchen entry")
        .g = 9;
    chain.write_atomic(&dir).expect("second write");
    match Chain::read(&dir, codec.braids()) {
        SidecarRead::Read(got) => assert_eq!(got, chain),
        other => panic!("expected Read, got {}", other.identity()),
    }

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
    assert!(matches!(
        Chain::read(&dir, codec.braids()),
        SidecarRead::Absent
    ));
}
