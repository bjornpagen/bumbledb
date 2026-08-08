# tests/

The SDK's four test zones (TODO_CPP §4):

- `cookbook/` — the 32 ported cookbook recipes, the cross-host conformance
  corpus (§33): each recipe's schema must lower through Rust to its golden
  in `fixtures/cookbook-fingerprints.txt` at the repository root.
- `compile_fail/` — the expected-failure compiler harness (§34): each case
  is one translation unit that must fail to compile AND emit a pinned
  diagnostic substring.
- `bridge/` — raw C ABI tests independent of reflection (§35): "is the
  foreign bridge correct?".
- `runtime/` — ordinary runtime unit tests of the C++ modules.

Reflection-importing tests are excluded from the Clang lint graph, mirroring
the `meta/` zoning.
