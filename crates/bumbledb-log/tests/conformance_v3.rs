//! Identity-table agreement across the native enums and both checked-in
//! copies. `cargo run -p bumbledb-log --bin identities` emits the table for
//! `conformance/v3/identities.json` and `ts/crate/log-identities.json`.

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
         `cargo run -p bumbledb-log --bin identities`"
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
         identities.json bytes"
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
