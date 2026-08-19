# 14 — V8 external-memory accounting misses lazy image birth

- **Status:** **fixed this pass** — heap `InstanceKind` carries
  `accounted: *const Cell<i64>`; scan/get/contains/prepare/execute/explain
  sync `retained_bytes()` after the call. Close releases the cell.
- **Severity:** should-fix.
- **Supersedes:** the remainder of BND-02.

## Principle

The proposal's host gate: "V8 external-memory accounting rises **and falls**
with frozen catalog **and native image capacity**." Accounting only the
admit-time snapshot makes the number a birth certificate, not a
representation of retained bytes — the lazily built relation images (the
large, demand-driven allocations) stay invisible to GC pressure.

## Evidence

- `ts/crate/src/lib.rs` — `account_bytes` on `AdmitTask::resolve` (frozen
  catalog + images already built), `release_accounted` on
  `owned_instance_close`. No adjustment where a frozen image slot fills
  later.

## The fix

1. The frozen image slot fill (`OnceLock` set on first bind) reports its
   image bytes to the owning handle's accounted total. The natural seam:
   `OwnedSlot` carries `accounted: Cell<i64>`; the bind path returns
   newly-built bytes; the napi entry that triggered the bind adjusts and
   bumps the cell.
2. `owned_instance_close` releases the *current* cell value (it already
   releases; the cell just makes the number true).
3. No adjustment on the JS thread's hot path beyond the first fill — the
   `OnceLock` already makes "first fill" a one-time event per relation.

## Acceptance

- Host gate test: external memory rises after the first execution that
  builds a lazy image, and falls by the same total on dispose.
- The number equals `retained_bytes()` re-queried after the build (one
  source of truth; the cell mirrors the engine's own count).
