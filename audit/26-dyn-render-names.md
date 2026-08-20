# 26 — `&dyn Names` × 8 in schema rendering

- **Status:** **fixed this pass** (re-verified 2026-08-19 — 7 `&dyn Names`
  sites at `schema/render.rs:468,586,603,614,624,633,679`; the filed ×8
  overcounted. All genericized to `N: Names + ?Sized`. `grep "dyn Names"
  crates/` empty. Render goldens byte-identical:
  `goldens_render_the_exact_macro_notation` and the 10 sibling
  `schema::render` tests.)
- **Severity:** zero-dyn law (cold path — law compliance, not perf).

## Principle

The law is a census, and a census with cold-path exceptions is a judgment
call per line forever. Rendering is cold, but `N: Names` monomorphizes at
zero design cost, so the exemption would buy nothing except a longer
allowlist.

## The fix

Genericize every `&dyn Names` parameter to `N: Names + ?Sized` (or plain
generics — two `Names` impls exist at most: id-spelled and name-resolved).
No behavior change; renders byte-identical (golden tests already pin them).

## Acceptance

- `grep "dyn Names" crates/` empty; render goldens unchanged.
