# Log identity golden

`identities.json` is the generated roster of current log identity domains,
frame tags, and protocol outcomes. Its byte-identical native-bridge copy is
`ts/crate/log-identities.json`.

`crates/bumbledb-log/tests/conformance_v3.rs` compares both files with a fresh
`bumbledb_log::identities::emit()` result. The `identities` binary emits the
same JSON for deliberate regeneration when the roster changes.

The directory's `v3` name is historical; it is not the current protocol
version. The retired braid, lease, counter, batch, and checkpoint corpus is
not a specification of the current log. Current codecs and authority tests
live with the implementation under `crates/bumbledb-log/` and `ts/crate/`.

A matching golden proves a stable roster, not crash safety, S3 behavior, or
cross-platform execution. Those require their respective tests.
