# 13 — The C callback exit still smuggles an `Io(Interrupted)` sentinel through the engine

- **Status:** **keep** (one `Result<()>` channel) / **fixed this pass**
  (unforgeable hatch). `Error::hatch` / `downcast_hatch` carry a
  bridge-owned ZST (`CallbackDecline`); family is still `ErrorFamily::Io`
  — no new ABI kind. `grep -n "ErrorKind::Interrupted" crates/bumbledb-c/src/`
  is empty. Tests: `hatch_reuses_io_family_and_downcasts`,
  `genuine_io_is_not_the_hatch`,
  `abort_plus_engine_failure_reports_engine_failure`,
  `abort_plus_hatch_is_aborted`, `write_abort_commits_nothing`.
- **Severity:** should-fix.
- **Supersedes:** BND-04.
- **Adjudication (third pass): keep ACCEPTED in principle; the fix
  NARROWS rather than closes.** The one-channel argument is right: the
  write body is `Result<()>`, abort must ride it, and threading
  `Result<R, Exit>` through the engine would be a second write algebra.
  What remains wrong is the *spelling* of the rider: `Io(Interrupted)` is
  an ambient kind the engine could legitimately produce, so a genuine
  engine interrupt in the same frame is swallowed into a clean
  `BDB_STATUS_ABORTED`. Narrowed fix: an unforgeable decline value — a
  `#[doc(hidden)]` engine variant (or a bridge-owned zero-sized error type
  behind `Error::External`) that only bridges mint and only bridges match,
  never mappable from real I/O. One channel, zero ambiguity, no new
  ErrorFamily arm crossing the ABI.

## Principle

Insight 5/6: the sentinel is an in-band error value that every layer between
the callback and the status map must know not to trust — `Exit` exists as
the sum, but the closures still return the magic `io::Error` so the engine
frame unwinds, and `is_callback_interrupt` re-parses it on the way out.

## Evidence

- `crates/bumbledb-c/src/db.rs:363-368` — `callback_interrupt()` mints
  `Error::Io(ErrorKind::Interrupted)`; `is_callback_interrupt` matches it
  back.
- The risk it carries: any *genuine* engine `Io(Interrupted)` raised in the
  same frame is indistinguishable from a host abort and reports as a clean
  `BDB_STATUS_ABORTED`.

## The fix

Thread `Exit` as data end-to-end: the callback wrapper returns
`Result<R, Exit>` (or the write body returns `ControlFlow`-shaped data the
bridge already defined), so the abort never enters the engine's error
channel at all. `callback_interrupt` and `is_callback_interrupt` delete; a
real `Io(Interrupted)` from the engine surfaces as itself.

## Acceptance

- `grep -n "Interrupted" crates/bumbledb-c/src/` is empty.
- Abort-path test: a callback abort during which the engine also fails
  reports the engine failure, not `ABORTED` (the swallow the sentinel made
  possible is pinned dead).
