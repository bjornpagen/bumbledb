# Packed public consumers (D07 / D22 / D27)

These are the installed-package specimens: Rust core, TypeScript core,
TypeScript log, and the native-ledger-shaped application. They compile
against the **shipped** surfaces — never workspace path aliases, `#`-import
conditions, or repo compiler settings.

Verification: **NotRun** until packed-consumer qualification with freshly
installed tarballs. Authoring these files is not evidence they executed.

- `core-ts/consumer.ts`, `log-ts/consumer.ts`, `native-ledger/consumer.ts`
  — copied by `scripts/packed-import.sh` into an isolated tarball consumer.
  They construct lazy programs and export them; import performs no native
  work. Runtime lifecycle lives in `scripts/packed-consumer.ts`.
- `rust/` — a standalone Cargo package outside the workspace. Crate
  publication is not authorized (`publish = false`); a path dependency is
  the installed-consumer stand-in:
  `cargo run --manifest-path examples/consumers/rust/Cargo.toml`.

All four spell the same `Learning` schema so schema-identity gates can
compare one fingerprint across public surfaces.

## Journey L21 must run

1. Pure import of schema/query/`Scalar.add(Scalar.field("units"), Scalar.u64(1n))` with the native package unavailable.
2. Install fresh packed core/log/native artifacts outside the workspace.
3. Generate history, initialize, mutate with sealed IDs, same-ID retry/resolve, witnessed correction.
4. Reuse `readAttempts` on a core snapshot and a published snapshot.
5. `collect` / `pages` under explicit delivery work; tiny `resultBytes` must refuse.
6. Generated increment-units convert, reopen, backup/restore, joined close.
7. Public Rust consumer: `ChangeSet::builder(db.schema(), work.clone())` + `db.apply` + `ApplyOutcome::InvariantRejected` + `db.snapshot(&work)` + `Db::close() -> CloseReport`.
8. Generated runner input is `{ manifest, plans, snapshots }` (empty-base plus one snapshot per entry).

## What these files delete

Old replica/`Promise`/callback APIs, private imports, handwritten plan
bytes, runtime-per-request, unlimited work twins, and fake successful
outcomes. Lost ack is resolved under the original command identity.
