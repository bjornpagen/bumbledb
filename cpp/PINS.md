# PINS — the pinned-quirk registry

One entry per pinned toolchain quirk. Every `/* PIN(name) */` in the tree
points at an entry here; the essay lives here, once. Tombstone ritual: on
every toolchain bump, read this file top to bottom, re-test every
tombstone, delete what upstream fixed — one file, one sweep.

Pinned toolchain: GCC 16.1.0 (production), LLVM 22 clang-tidy (lint
graph).

## gcc-partition-bmi-expected

- symptom: a NON-template member function definition whose body
  instantiates the foreign `std::expected` API corrupts the `:db`
  partition's BMI for re-export — the primary interface's
  `export import :db;` dies with "failed to read compiled module
  cluster N: Bad file data". Template members are unaffected.
- sites: `src/db/db_impl.cc` — the file's entire reason to exist, the
  module's one interface/impl split, forced by the quirk and not by
  design. It holds the bodies of `Db::admit`, the pre-schema
  `Db::create` / `Db::open` / `Db::ephemeral` lanes, and
  `Db::fingerprint`, all declared in `src/db/db.cc`.
- workaround: move those bodies into a module implementation unit — an
  implementation unit produces no BMI, so there is nothing to corrupt.
- retire: on any GCC bump, fold the bodies back into `db.cc` and delete
  `db_impl.cc` once the re-export streams clean.
- upstream: not filed — no standalone repro yet, and three open "Bad
  file data" reports with standalone repros are already in the queue
  (PR 125595, same symptom, reduced upstream to VLA streaming in an
  inline function; PR 125144; PR 125356 — cite all three when filing).
  A "me too" without a repro is not actionable. Reduction ledger
  (2026-08-09, absorbed from cpp-starter's deleted
  upstream/gcc-modules-partition-bmi-expected/): plain
  partition + `<expected>` GMF shapes do NOT reproduce; neither does
  the payload-type-from-imported-module shape; the "`import std` is
  the missing ingredient" hypothesis was tested and falsified. The
  faithful reproduction remains only this tree: move the five bodies
  from `db_impl.cc` back into `db.cc` and build the dev preset. Next
  reduction step: a scratch CMake project mirroring the real
  two-module graph (`bumbledb` importing `bumbledb_foreign`) with the
  actual five member bodies, then shrink creduce-style keeping the
  three-file structure — the full foreign-module context is the
  remaining suspect. File to Bugzilla (component c++, [modules]) only
  once a standalone repro exists.

## gcc-template-for-wshadow

- symptom: expansion statements (`template for`) trip `-Wshadow` on
  compiler-generated scoping whenever the expanded range has more than
  one element. Expansion happens at the template INSTANTIATION site, so
  every TU that instantiates the reflective templates trips, not just
  the partitions that define them.
- sites: scoped `-Wno-shadow` on the `bumbledb` module target
  (`src/CMakeLists.txt`), on every runtime test target
  (`tests/runtime/CMakeLists.txt`), on every cookbook target
  (`tests/cookbook/CMakeLists.txt`), and on every compile-fail case
  (`tests/compile_fail/CMakeLists.txt`, where it also keeps a case's
  only diagnostic the one under test).
- workaround: the scoped per-target `-Wno-shadow` — never an inline
  suppression.
- retire: on any GCC bump, drop the option from one reflective target
  and rebuild; delete it everywhere when the warning stays quiet.
- upstream: not filed (pinned GCC 16.1.0).

## ubsan-constexpr-string

- symptom: under `-fsanitize=address,undefined`, `std::string`'s
  (pointer, size) constructor carries a null check that does not
  constant-fold against ASan-instrumented storage (template parameter
  objects, string literals, `define_static_string` globals), so
  consteval name synthesis fails as "not a constant expression". The
  iterator-pair constructor folds.
- sites: `bdb::detail::spec_name` (`src/relation/name.cc`) — the single
  funnel, built with `std::string(text.begin(), text.end())`. Every
  injected `data_member_spec` name computed from a `name_text` or
  derived view routes through it (callers in `src/closed/facade.cc`,
  `src/schema/key.cc`, `src/schema/schema.cc`, `src/query/query.cc`);
  names straight from `identifier_of` (reflection-internal storage)
  need no detour.
- workaround: construct the name payload through the iterator-pair
  constructor, and keep every synthesized name routed through the one
  funnel.
- retire: on any GCC bump, respell `spec_name`'s body as the
  (pointer, size) construction and run the asan-ubsan preset.
- upstream: GCC PR 71962.

## reflect-using-decl

- symptom: `^^` applied through a standard library alias is ill-formed
  on the pinned GCC — `^^std::uint64_t` dies with "'^^' cannot be
  applied to a using-declaration".
- sites: `bdb::detail::type_reflection` (`src/relation/classify.cc`)
  and `bdb::detail::query_type_reflection` (`src/query/query.cc`) —
  variable templates whose only job is the detour.
- workaround: route the type through a template parameter
  (`template<class T> inline constexpr auto x = ^^T;`) so the alias
  resolves during substitution before `^^` applies — conforming, not a
  dialect violation.
- retire: on any GCC bump, try `^^std::uint64_t` directly and delete
  both helpers if it compiles.
- upstream: P2996R13 (reflection for C++26); GCC's in-progress P2996
  implementation, not filed separately.

## llvm22-unused-using-decls

- symptom: clang-tidy's `misc-unused-using-decls` does not recognize an
  exported using-declaration as a named module's export surface, so
  every `export using ::bdb_*;` function re-export in
  `foreign/bridge.cc` is a false positive ("unused").
- sites: the `bumbledb_foreign` target in the lint graph
  (`foreign/CMakeLists.txt`): `--checks=-misc-unused-using-decls`
  appended to `CXX_CLANG_TIDY` on exactly this target — every other
  check still runs over the TUs.
- workaround: the scoped single-check accommodation on the single
  target.
- retire: re-test on any clang-tidy bump; delete the accommodation when
  the check learns module export surfaces.
- upstream: LLVM 22 clang-tidy, not filed.

## compile-fail-vs-sanitizers

- symptom: `-fsanitize=address,undefined` breaks constexpr
  `std::string` evaluation inside the reflector's `static_assert`
  message machinery ("is not a constant expression" instead of the
  pinned diagnostic), so instrumented graphs cannot reproduce the
  pinned compile-fail text. Same root family as ubsan-constexpr-string.
- sites: `tests/CMakeLists.txt` — `compile_fail/` is added only when
  `CMAKE_CXX_FLAGS` carries no `-fsanitize` (and never in the lint
  graph, which does not drive the production compiler).
- workaround: pinned diagnostics are a product of the canonical
  production configurations (dev/release), where the suite runs; CI
  keeps compile-fail as its own uninstrumented job.
- retire: on any GCC bump, configure an instrumented graph with the
  suite included and check the pinned diagnostics reproduce.
- upstream: GCC PR 71962 (the constant-folding root); the exclusion is
  the suite-level face of it.
