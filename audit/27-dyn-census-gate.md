# 27 — The zero-dyn census gate

- **Status:** **fixed this pass** — `zero_dyn_engine_pins_error_source_exemption`
  plus `scripts/spec-census.sh` (g) pin engine `dyn` to the three
  `Error::source` / `ErrorDescriptor` lines. Temporary revert of 24
  (`dyn FnMut` in `judgment.rs`), 25 (`dyn Any` in `error.rs`), and 26
  (`dyn Names` in `render.rs`) each reddened the gate; restored.
- **Severity:** law pin.

## The law

The engine crates (`bumbledb`, `bumbledb-theory`, `bumbledb-query`,
`bumbledb-macros` emission) contain **no `dyn`** except where a `std` trait
signature mandates it. The SDK crates (`ts/crate`, `bumbledb-c`) may use
`dyn` freely — bridges erase types for hosts; that is their job.

## The one written exemption

`std::error::Error::source(&self) -> Option<&(dyn std::error::Error + …)>`
— the signature is std's, not ours (`error.rs` impl + the `ErrorParts`
mirror that feeds Display). Exemption list: exactly those lines.

## The fix

A census test (same idiom as `spec-census.sh` / the two-module
`FilterPredicate` gate): enumerate `dyn` occurrences in the engine crates,
assert the set equals the pinned exemption list. A new `dyn` fails the
suite with the file:line in the message. Wire it into `spec-census.sh` so
the three-way gate carries it.

## Acceptance

- The census test exists, is green, and pins the exemption list.
- Deleting any of 24/25/26's fixes turns it red (verified once by
  temporary revert during development).
