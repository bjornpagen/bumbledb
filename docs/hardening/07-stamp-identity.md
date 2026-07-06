# PRD 07 — The stamp knows what it vouches for

Findings fixed (docs/audit/oracle.md): **HIGH** "The verify stamp does not
invalidate on code changes — bench brands unverified engines VERIFIED";
**LOW** "Divergence-by-error is a panic, not a mismatch bundle"; **LOW** "A
NUL byte in a String literal silently truncates the SQL statement"; **NOTE**
"Family param functions are outside the family digest" (subsumed); **NOTE**
"A stamp can be earned with zero randomized cases".

## Purpose

The verification stamp is the mechanism behind success criterion 1 — and it
currently hashes nothing that identifies the code it vouches for. One binary
contains the engine, translator, comparator, generator, and param policies;
hash the binary, and every one of those is covered at once. While in the
verify loop: a one-sided error (engine errors where SQLite answers, or vice
versa) must be an arbitration artifact, not a panic — the audit confirmed a
real semantic divergence class (SQLite's transient SUM overflow vs our i128
accumulator) that today would abort verify bundle-less.

## Technical direction

- **Binary fingerprint in the stamp.** `verify.rs::stamp_value` folds
  `blake3(read(std::env::current_exe()))` — computed once per process
  (a `OnceLock<[u8;32]>` in the bench crate; expose as
  `verify::binary_fingerprint()`), replacing the `CARGO_PKG_VERSION`
  ingredient outright (cutover; the version string identified nothing).
  Consequences, stated in the module docs: any rebuild re-keys the stamp
  (over-invalidation by embedded paths is accepted — re-verification is the
  honest default); `stamp_matches` therefore fails for any binary other than
  the one that earned the stamp, which is precisely the contract. The
  param-functions NOTE is subsumed: params are code; code is the binary.
- **Divergence-by-error becomes a bundle.** `verify.rs::Run::check`
  (`:120-129`): the four `expect`s on prepare/execute (both engines) become
  handled outcomes. New comparison result: if exactly one side errors, that
  is a **mismatch** — write the standard bundle with the erring side's error
  text in place of its rows (`ours.txt`/`theirs.txt` gain an
  `ERROR: <display>` form), continue collecting, no stamp. If *both* sides
  error, record it as agreement-in-error only when the case is expected to
  error (there is no such case today — treat both-error as a mismatch bundle
  too, with both texts; a tool defect should not look like verification).
  Setup errors (store open, corpus load) stay panics — they are tool
  failures, not divergences.
- **NUL-safe string literals.** `translate.rs::sql_string_literal`
  (`:33-36`): a `\0` in a valid-UTF-8 literal truncates SQLite's tokenizer.
  Reject with a typed translator error naming NUL (the translator is total
  over the *generator's* grammar, which never emits NUL — keep it that way
  and make the boundary loud), and have querygen's docs note the exclusion.
  (The alternative — `CAST(X'..' AS TEXT)` — buys generality nobody
  generates; take the named error.)
- **A floor under the evidence.** `cli.rs`/`driver.rs`: `verify --cases 0`
  still writes a stamp bench accepts. Keep it legal (families-only
  verification is honest and the stamp encodes the count) but surface it:
  the bench report's provenance line gains the verified case count next to
  the stamp (`verify stamp: <hex> (families + N randomized cases)`), read
  from the `verify.cases` sidecar. Re-pin the report goldens. No hidden
  minimum — visibility over policy.

## Non-goals

Git-based identity (the binary hash strictly dominates: it covers dirty
trees, dependency bumps, and toolchain changes that a rev string misses);
process-local stamps (the file-based flow with the digest-keyed corpus dir
stays — it is now sound because the stamp is binary-bound).

## Passing criteria

- Stamp identity test: `stamp_value` differs across two byte-different
  binaries — simulate by asserting the fingerprint ingredient equals
  `blake3` of the current exe and that flipping it flips the stamp
  (`stamp_matches` rejects a stamp computed with a different fingerprint
  injected through a test seam). Plus the sidecar flow: a stamp earned by
  this binary is accepted by this binary (the driver e2e test extends).
- Divergence-by-error test: extend `run_with_sql_override` usage — inject
  SQL that *errors* on execution (e.g. `SELECT SUM(...)` over a crafted
  overflow, or simply invalid-at-execute SQL) for one family → the run
  returns `VerifyFailure`, the bundle exists with the `ERROR:` artifact, no
  stamp written, and the process did **not** panic.
- NUL test: `translate` on a query with a `\0`-bearing string literal returns
  the named error (not truncated SQL); querygen's coverage test asserts its
  grammar never emits NUL (a property assert over the generated literals).
- The report provenance renders the case count (goldens re-pinned); a
  `--cases 0` run's report visibly says `0 randomized cases`.
- `scripts/check.sh` green; the full-S verify test green with the new stamp.
