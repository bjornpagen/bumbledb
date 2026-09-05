//! The generated identity table's checked-in goldens: a fresh emission from
//! the successor enums must byte-equal both checked-in copies. The 0.x
//! braided corpus (braids/chain/counter/lease/batch/scratch goldens) is
//! deleted machinery; its inventory is retired with it (recorded in
//! implementation/packets/P05.md, corpus deletion is a P00 hub decision).
//!
//! REGENERATION (F3, executes code — deferred by the F1 rule): run
//! `cargo run -p bumbledb-log --bin identities` redirected into
//! `crates/bumbledb-log/conformance/v3/identities.json`, and copy the
//! identical bytes to `ts/crate/log-identities.json` (P06R's tree). Until
//! that runs, these tests are the red forcing function.
//! Verification: `NotRun` (F1 authors, does not execute).

use std::path::Path;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn checked_in_identity_golden_matches_a_fresh_emission() {
    let golden = manifest_dir().join("conformance/v3/identities.json");
    let checked_in = std::fs::read_to_string(&golden).expect("identities.json exists");
    assert_eq!(
        checked_in,
        bumbledb_log::identities::emit(),
        "conformance/v3/identities.json is stale; regenerate with \
         `cargo run -p bumbledb-log --bin identities` (F3)"
    );
}

#[test]
fn the_ts_crate_twin_is_byte_identical_to_the_same_emission() {
    let twin = manifest_dir().join("../../ts/crate/log-identities.json");
    let checked_in = std::fs::read_to_string(&twin).expect("ts/crate/log-identities.json exists");
    assert_eq!(
        checked_in,
        bumbledb_log::identities::emit(),
        "ts/crate/log-identities.json is stale; copy the regenerated \
         identities.json bytes (P06R tree, F3)"
    );
}

#[test]
fn the_emission_has_no_retired_families_and_every_kind_is_camel_case() {
    let emitted = bumbledb_log::identities::emit();
    // The generator's comment line names the deleted families to say they are
    // gone; every OTHER line is roster content and must not spell them.
    let roster: String = emitted
        .lines()
        .filter(|line| !line.trim_start().starts_with("\"comment\""))
        .collect::<Vec<_>>()
        .join("\n");
    for retired in [
        "braid",
        "vector",
        "sidecar",
        "lease",
        "splitOutcome",
        "deposition",
    ] {
        assert!(
            !roster.contains(retired),
            "retired 0.x family `{retired}` must not reappear in the roster"
        );
    }
    for family in [
        "\"frame\"",
        "\"admissionRefusal\"",
        "\"authority\"",
        "\"logError\"",
        "\"submitOutcome\"",
        "\"resolveOutcome\"",
        "\"conditionalOutcome\"",
        "\"putOutcome\"",
    ] {
        assert!(emitted.contains(family), "missing family {family}");
    }
}
