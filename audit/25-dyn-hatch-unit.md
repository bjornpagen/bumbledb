# 25 — The callback-decline hatch carries `Arc<dyn Any>`; it becomes a concrete unit

- **Status:** **fixed this pass** — `Hatch` is a doc-hidden unit (`Error::hatch` / `is_hatch`, no `Any`, no `Arc`); tests: `hatch_reuses_io_family_and_downcasts`, `genuine_io_is_not_the_hatch`, `abort_plus_engine_failure_reports_engine_failure`, `abort_plus_hatch_is_aborted`, `write_abort_commits_nothing`.
- **Severity:** zero-dyn law. Rides the purge lane (it owns `error.rs`).

## Principle

The one-channel ruling from 13 stands: abort rides the one `Result<()>`
write channel. But the rider's payload over-solved unforgeability: `dyn Any`
plus downcast buys "any bridge can smuggle any type," when the requirement
was only "not mintable from real I/O." A doc-hidden concrete unit variant is
unforgeable-from-I/O by construction, allocates nothing, dispatches nothing.

## Evidence

- `crates/bumbledb/src/error.rs:1650` — `Hatch(Arc<dyn Any + Send + Sync>)`
  with `Error::hatch` / `downcast_hatch`.
- Consumers: the C bridge's `CallbackDecline` mint/match (tests:
  `hatch_reuses_io_family_and_downcasts`, `genuine_io_is_not_the_hatch`,
  `abort_plus_engine_failure_reports_engine_failure`).

## The fix

Replace the `Any` machinery with a `#[doc(hidden)]` concrete decline value —
one unit type the engine defines and never mints on any engine path (a
constructor doc-contracted to bridges). `ErrorFamily` mapping unchanged
(still `Io`-family, no ABI kind). The bridge's match becomes an equality on
the variant, not a downcast. All five fix-13 tests re-point and stay.

## Acceptance

- `grep "dyn Any" crates/bumbledb/src` empty; the zero-dyn census passes.
- The five decline tests green under the new spelling;
  `genuine_io_is_not_the_hatch` still proves a real `Io` cannot be mistaken
  for the decline.
- No `Arc` allocation on the abort path.
