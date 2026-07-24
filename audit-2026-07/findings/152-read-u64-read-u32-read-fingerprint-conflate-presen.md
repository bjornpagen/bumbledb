## Meta readers conflate present-but-mis-sized values with absent keys as MetaMissing, contradicting the taxonomy read_store_kind pins in the same file

category: incoherence | severity: low | verdict: CONFIRMED | finder: r2:crash-recovery-lifecycle

### Summary

`read_meta.rs` contains two contradictory decode disciplines for the `_meta` keys. `read_store_kind` splits "key absent" (`MetaMissing`) from "key present but undecodable" (`StoreKindInvalid`), and both its doc comment and the error-variant doc declare that split semantically load-bearing: "corrupt data, not a missing key." Its four siblings — `read_u64` (tx id), `read_u32` (format version, via `check_format_version`), `read_fingerprint`, and transitively `read_dict_next_id` — funnel a present-but-wrong-width value into `MetaMissing`, the exact state the taxonomy says is *not* a missing key. The conflation is not silent drift: `MetaMissing`'s own doc says "absent or malformed," and each reader repeats it. That makes the incoherence a contradiction between two documented contracts in the same error enum, applied key-by-key inconsistently within one file — a representation problem (one error value encoding two opposite forensic states) of the kind the project's design doctrine (`docs/design/representation-first.md`) exists to erase.

### Evidence (all verified in the working tree)

- `crates/bumbledb/src/storage/env/read_meta.rs:13-16` (`read_u64`), `:25-28` (`read_u32`), `:107-109` (`read_fingerprint`) — identical shape:
  ```rust
  meta.get(rtxn, key)?
      .and_then(|b| b.try_into().ok())
      .ok_or(Error::Corruption(CorruptionError::MetaMissing))?
  ```
  A 7-byte value survives `get` as `Some`, fails `try_into`, and reports `MetaMissing`.
- `crates/bumbledb/src/storage/env/read_meta.rs:54-73` — `read_store_kind`, the one correct implementation: absent key => `MetaMissing` (line 68); present but wrong-width or unknown byte => `StoreKindInvalid` (line 72), with the doc stating the rationale ("corrupt data, not a missing key").
- `crates/bumbledb/src/error.rs:42-49` — the contradiction lives in the variant docs themselves: `MetaMissing` = key "absent **or malformed**" (42-44), immediately followed by `StoreKindInvalid` = "Distinct from `MetaMissing`: the key exists, so this is corrupt data, not a missing key" (45-49).
- Real reopen/forensics callers of the conflating readers: `readtxn.rs:19` (`ReadTxn::generation` reads `META_TX_ID`), `writetxn.rs:63` (write-side generation), `readtxn.rs:34` and `exhume.rs:71` (`read_fingerprint` in the schema-less exhume entry), `read_meta.rs:44` (`check_format_version`, first in the open-time check precedence).
- `CorruptionError::MalformedValue(&'static str)` (error.rs:92-95) already exists and is already the convention for present-but-undecodable stored values — used in the same file by `read_dict_next_id` (read_meta.rs:121) and by `stored_u64` in `storage.rs:16-18`, so the repo has two u64 decoders with opposite contracts for the same failure.
- Test coverage confirms the asymmetry is untested on the conflating side: `storage/env/tests.rs:226-294` covers absent store-kind => `MetaMissing`, wrong-width/unknown store-kind => `StoreKindInvalid`; `tests.rs:439-461` covers *absent* fingerprint => `MetaMissing`. No test writes a mis-sized `META_TX_ID`/`META_FORMAT_VERSION`/`META_FINGERPRINT` value, so the conflated branch's behavior is pinned only by the contradictory doc comments.

### Failure scenario

A store whose `META_TX_ID` value was truncated by a torn write or bit rot: the first `ReadTxn::generation()` (or `WriteTxn` generation read, or exhume's fingerprint read for its key) raises `Corruption(MetaMissing)` — "this environment is not a usable bumbledb database / never initialized" — when the key is present and its value corrupt. The two states point at opposite recovery directions (re-adopt vs. investigate a torn write), and the error type is the only signal the operator or the `verify_store`/exhume lanes get. No data is lost; the diagnosis is misdirected.

### Suggested fix

One decode discipline for all meta values, matching `read_store_kind`: split the map — key absent => `MetaMissing`; present but mis-sized => `MalformedValue(what)` naming the key (`"tx id"`, `"format version"`, `"schema fingerprint"`, `"dict next id"`). `read_u64`/`read_u32` gain the `&'static str` parameter their `stored_u64` sibling in `storage.rs:16` already takes (unification: one u64-decode contract in the crate). Then fix the contradiction at its root: drop "or malformed" from the `MetaMissing` doc (error.rs:42-44) and the four reader doc comments (read_meta.rs:39, 79-80, 101-102; readtxn.rs:14; writetxn.rs:61), and add the missing wrong-width tests beside the existing store-kind matrix in `storage/env/tests.rs`. Optional follow-on: `StoreKindInvalid` becomes representable as `MalformedValue("store kind")`, collapsing the special case entirely.
