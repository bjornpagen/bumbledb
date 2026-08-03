## exhume never reads the R18 dirty marker: a crashed ephemeral store opens through the archival lane and serves possibly-torn pages as verified

incoherence | medium | CONFIRMED | storage-v7
outcome: fixed 15342f00

### Summary

R18 (the ephemeral crash contract) rules that the torn-store state is **unrepresentable, not detected**: "Every ephemeral open sets a synced dirty marker before trusting anything else... the possibly-torn store is never opened at all" (docs/architecture/50-storage.md:572-590), and the marker lives as a sibling file precisely because it "must be readable before any LMDB page is trusted" (crates/bumbledb/src/storage/env.rs:280-282). But `Environment::exhume` — an open that trusts LMDB pages of BOTH kinds — never consults the marker: grep confirms `dirty_marker_path` has exactly two consumers in the whole crate, the open-side wipe check in `ephemeral.rs:64-74` and the clear in `Environment::drop` (env.rs:266-278). The exact silent-corruption mechanism R18 was minted to close for the writing constructor stays open through the read-only one.

### Evidence

- `crates/bumbledb/src/storage/env/exhume.rs:56-98`: `open_env(ReadOnly)` → read txn → `check_format_version` → `read_store_kind` (line 66 — the ephemeral kind byte is read and *reported*, never acted on) → fingerprint + descriptor reads → handle constructed with `dirty_marker: None` (line 92). No `dirty_marker_path` call anywhere on the path.
- `crates/bumbledb/src/api/db/exhume.rs:61-91`: both integrity gates cover **meta bytes only** — blake3 of the stored descriptor vs the stored fingerprint (lines 63-71) and the decode/re-encode round trip (lines 72-84). No data-tree page is examined; `Exhumed::read` (lines 147-152) then scans the data trees directly. `Exhumed::kind` (lines 113-119) reports `Ephemeral` but nothing reports "crashed".
- The state is real by the repo's own spec: 50-storage.md:584-587 — "a meta page flushed by incidental writeback over data pages that never landed — fingerprint-valid over trees no committed transaction ever contained." That sentence is the whole justification for R18; exhume's gates are exactly the checks it declares insufficient.
- No doc carves exhume out: 70-api.md § exhume's "reads BOTH kinds" clause (line 456-458) is about kind *comparison* (no `StoreKindMismatch`), not the crash marker, which the section never mentions. The R18 text says "every ephemeral open" with no read-only exception.
- Test coverage gap: the marker matrix in `storage/env/tests.rs:53-150` covers clean close, planted-marker wipe, and durable-never-mints; `api/db/exhume/tests.rs:244` exhumes an ephemeral store only after a **clean** close (marker already cleared). No test exhumes a marker-set store.

### Failure scenario / impact

A machine loses power mid-session on an ephemeral store: marker set, NOSYNC data pages torn, meta pages incidentally flushed. An operator — knowing `Db::ephemeral` would wipe — reaches for the archival lane to salvage the record: `bumbledb::exhume(path)`. Format version, kind, fingerprint, descriptor hash, and round trip all pass (meta is intact). `Exhumed::read` scans then either raise mid-ETL `Corruption` (best case) or yield generation-mixed rows no committed transaction ever contained — which the documented rebirth pattern (70-api.md:453-454: exhume old store → copy into successor) feeds into `bulk_load_dyn` (api/db/write.rs:371) on a **durable** store with no error anywhere. Silent corruption crosses from the kind that renounced durability into the kind that promises it.

### Suggested fix

One `try_exists` at the top of `Environment::exhume`: a set marker refuses typed (new variant, e.g. `Error::EphemeralUnclean`, naming the remedy — reopen via `Db::ephemeral` to wipe, or accept the record as lost). Read-only media are unaffected: a cleanly-closed store has no marker, so the check reads the directory only. Record the deliberate trade in the rustdoc: a LIVE ephemeral writer in another process also holds a set marker, so cross-process exhume-while-open of an ephemeral store becomes a refusal — consistent with R18, since the lockless read lane (R17) has no way to distinguish live from crashed. Pin with a marker-planted exhume test beside the existing matrix in `storage/env/tests.rs` (and a live-session refusal twin if the trade lands).