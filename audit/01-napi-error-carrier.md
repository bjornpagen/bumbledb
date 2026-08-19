# 01 — The napi throw carrier is a bare object; the TS suite is red

- **Status:** **fixed this pass** — `throw_object` is `create_error` + `kind`; `errorFromThrow` keeps a real `Error`. Open/prepare refusals wrap `ErrSchemaError` / `ErrFingerprintMismatch` / `ErrIrError` / `ErrNewtypeMismatch`.
- **Severity:** ship-blocker — `pnpm test` fails.
- **Supersedes:** VER-01, the carrier half of BND-01, BND-07.

## Principle

Insight 6 (parse, don't validate) and Insight 5. The error family table is the
parse — done. But the *carrier* discards what JavaScript already gives us: an
`Error` is an object, so the kind can ride a real `Error` without displacing
the message, the stack, or `instanceof`. Throwing a bare `{ kind, message }`
object makes `.message` unreachable and turns every message-matching consumer
into `Error: [object Object]`.

## Evidence

- `ts/crate/src/marshal.rs:74-79` — `throw_object` builds `Object::new`,
  sets `kind` and `message`, and `env.throw(obj)`s it.
- Reached from `throw_kind_message` (`marshal.rs:63`), `throw_engine`
  (`marshal.rs:56`), and `AdmitTask::resolve`'s `Failed` arm
  (`ts/crate/src/lib.rs:1334-1336`).
- Failure: `ts/test/query.test.ts:484` — `assert.throws(fn, /expected
  Interval/)` receives `Error: [object Object]`. Suite result: exactly one
  failing test (`point membership: literal, param (both value shapes), and
  pointIn`).

## The fix

One carrier: a real `Error` with the kind as a property.

1. `throw_object` constructs `new Error(message)` (napi:
   `Error::new`/`create_error`) and sets `kind` on the error object before
   throwing — `err.kind = tag`. Message, stack, and `instanceof Error` all
   survive; kind-narrowing consumers read `err.kind`.
2. `ts/src/native.ts` `errorFromThrow` keeps parsing `{ kind, message }` —
   it now also matches plain `Error`s carrying `.kind` (one shape, both
   sides).
3. BND-07's tail rides along: `throwOpenRefusal` and the prepare error paths
   in `ts/src/db.ts` narrow on `err.kind` (or wrap the exported `Err*`
   values); no `errors.new(\`bumbledb ${kind}…\`)` string assembly.

## Single way

One thrown shape everywhere: `Error & { kind?: ErrorFamilyTag }`. Bridge-only
refusals (closed handle, re-entrancy) stay exported named values without a
`kind` — they are not engine families and must not impersonate them.

## Acceptance

- `pnpm test` green.
- `assert.throws(fn, /expected Interval/)` passes unchanged — message
  matching works again.
- A kind-narrowing test pins `err.kind === "factShape"` (any family) on an
  engine refusal.
- No `env.throw` of a non-`Error` object anywhere in `ts/crate`.
