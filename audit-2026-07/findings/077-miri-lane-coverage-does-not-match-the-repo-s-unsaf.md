## Miri lane exclusion ledger: arena (and digest) mis-filed as FFI; ts crate's unsafe surface silently out of scope

category: incoherence | severity: medium | verdict: CONFIRMED | finder: r2:concurrency-unsafe-ffi

### Summary

The Miri lane (`scripts/miri.sh`) is built as an honest allowlist with a per-exclusion reason ledger in its header. Two entries of that ledger fail verification:

1. **Mis-filed pure modules.** `arena` and `digest` are excluded under the reason "FFI: they open LMDB environments through heed/lmdb-sys" (miri.sh:27-35). Neither does. `arena.rs` opens with "No external crate, no `unsafe`" (crates/bumbledb/src/arena.rs:1-6) and its two tests touch only `Vec<u8>` (arena.rs:71-101); `digest.rs`'s single test is a pure blake3 streaming check (digest.rs:29-42) — and the header itself notes at lines 14-15 that blake3's portable body interprets fine under Miri (it already runs it via `encoding::tests`). I ran `cargo miri test -p bumbledb --lib -- arena::` on the pinned toolchain: **2 passed in 6.6s**. The coverage is free today; the risk is tomorrow — a future `unsafe` addition to arena (a bump arena is exactly the kind of module that grows one) would inherit an exclusion whose stated justification was never true for it.

2. **Undocumented ts-crate scoping.** The lane runs `cargo miri test -p bumbledb --lib` only (miri.sh:77, 97). The TypeScript bridge crate (ts/crate — a separate workspace with its own Cargo.lock) carries the repo's largest per-file unsafe surface: the prepared-query/snapshot pointer-laundering protocol (`unsafe { &*(prepared as *const PreparedQuery<'static, SchemaDescriptor>) }` at ts/crate/src/lib.rs:649, plus lib.rs:632, 943, 1257) and the napi `Unknown::cast` marshal sites (ts/crate/src/marshal.rs:186-230). No script, header, or doc records the decision to leave it out (`grep -rn miri` over ts/ and .github hits only the engine lane). The exclusion is largely inherent — this unsafe lives at the napi FFI boundary, and its Rust tests (fingerprint_lock) open real LMDB stores — but the engine lane documents every inherent exclusion with a reason, and the ts crate gets no line at all.

The finding's third leg — that `exec/kernel/neon.rs`, the one sanctioned hand-NEON unsafe module, is interpreted on **no** lane — is factually true and I reproduced it, but it is **documented and inherent**, so it is a coverage fact rather than an incoherence (see Evidence). The colt/decode `get_unchecked` exclusions are likewise real but already carry documented reasons (colt: miri.sh:33-35) and are the subject of a separate finding.

### Evidence (all verified directly)

- miri.sh:65-67 — FILTERS allowlist: `allen::tests:: interval::tests:: interval::sweep:: encoding::tests:: schema::tests::member_set exec::kernel::tests:: exec::wordmap:: ir::normalize::fold::tests::`. No `arena::`, no `digest::`.
- miri.sh:27-35 — the exclusion bin naming "…digest, arena, the tests/ integration binaries) — FFI: they open LMDB environments through heed/lmdb-sys".
- crates/bumbledb/src/arena.rs:1-6 ("No external crate, no `unsafe`"), tests at 71-101 (Vec-only). Ran under Miri: 2 passed, 6.6s.
- crates/bumbledb/src/digest.rs:29-42 — pure blake3 streaming test, no Db/TempDir/heed anywhere in the file.
- ts/crate/src/lib.rs:632, 649, 943, 1257 and ts/crate/src/marshal.rs:160-230 — the bridge's unsafe sites; miri.sh:77/97 scope the lane to `-p bumbledb --lib`.
- NEON leg, reproduced: crates/bumbledb/src/exec/kernel/allen.rs:214-225 dispatches to `neon::allen_code_batch_neon` on aarch64 and `reference::allen_codes` otherwise; the native pass skips `exec::kernel::tests::allen` (miri.sh:77-78) and the cross pass runs the reference twins. Removing the skip and running one allen test under the pinned Miri (nightly-2026-07-12) fails with `unsupported operation: can't call foreign function 'llvm.aarch64.neon.tbl4.v8i8' on OS 'macos'` — the script's stated reason at miri.sh:36-43 is accurate; the gap cannot be closed by lane configuration, and the header names the actual referee (the native bit-identity property test against `reference`, kernel/tests.rs:1071).
- Doctrine check: docs/architecture/40-execution.md and the kernel header (kernel.rs:8-23) sanction exactly one unsafe intrinsic module; the crucible packet's verdict matrix is the recorded justification. The lane's header structure (reason-per-exclusion) is itself the contract this finding holds it to.

### Failure scenario

Process-level, not runtime: the ledger's false "FFI" reason on arena/digest means a future unsafe addition to either module ships outside Miri under an exclusion that was never justified for it; and a reader auditing "is the unsafe surface Miri-clean?" finds no recorded answer for the ts bridge — the crate whose pointer-laundering protocol is the most regression-prone unsafe in the repo — because the scoping decision exists only implicitly in a `-p` flag.

### Suggested fix

Three one-line-scale edits to miri.sh, no behavior change beyond free coverage:
1. Move `arena::` and `digest::` into FILTERS (both pass under Miri today; verified for arena).
2. Correct the exclusion comment so the LMDB-FFI bin no longer names arena/digest.
3. Add one header line recording the ts-crate scoping: the bridge's unsafe is napi-boundary code Miri cannot interpret (same wall as mdb_*), refereed instead by the fingerprint lock and the SDK's node-test lane — mirroring the honesty the header already applies to NEON at lines 36-43.
