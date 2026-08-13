# PINS.md — the pinned-quirk registry

One entry per pinned workaround: code or build configuration that is
deliberately wrong by dialect law because the pinned toolchain or platform
requires it. Each `PIN(name)` site in the tree points at its entry
here; the essay lives here, once.

Tombstone ritual: on every toolchain bump, read this file top to bottom,
re-test every retire condition, and delete what upstream fixed — one file,
one sweep.

The accepted pinned toolchain release series and generator live only in the
top-level CMake configure gate.

## darwin-inert-mitigations

- symptom: `_FORTIFY_SOURCE=3` is excluded from every C++ TU by Apple's SDK,
  and `-mbranch-protection=standard` executes as NOP in plain-arm64 Darwin
  processes, so both flags are byte-for-byte inert on the dev host
- sites: CMakeLists.txt — Linux-only `if(BDB_LINUX)` block on
  `bumbledb_language_profile`; Darwin never receives the flags
- workaround: select the live mitigations in CMake, never behind a C++ `#ifdef`
- retire: enable each flag on Darwin when the SDK actually instruments C++
  (FORTIFY) or the process is PAC/BTI-enforced (branch protection); re-test
  on every toolchain bump
- upstream: none — platform ABI facts, recorded so they are not re-enabled
  as empty checks

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
  (2026-08-09): plain partition + `<expected>` GMF shapes do NOT
  reproduce; neither does the payload-type-from-imported-module shape;
  the "`import std` is the missing ingredient" hypothesis was tested
  and falsified. The faithful reproduction remains only this tree: move
  the five bodies from `db_impl.cc` back into `db.cc` and build the
  dev preset. Next reduction step: a scratch CMake project mirroring
  the real two-module graph (`bumbledb` importing `bumbledb_foreign`)
  with the actual five member bodies, then shrink creduce-style keeping
  the three-file structure — the full foreign-module context is the
  remaining suspect. File to Bugzilla (component c++, [modules]) only
  once a standalone repro exists.

## gcc-template-for-wshadow

- symptom: expansion statements (`template for`) re-declare the loop
  variable in a nested scope per iteration, so any expansion over a range
  with more than one element trips `-Wshadow` on compiler-generated
  scoping (fires once per element) — a build failure under `-Werror`.
  Expansion happens at the template INSTANTIATION site, so every TU that
  instantiates the reflective templates trips, not just the partitions
  that define them.
- sites: scoped `-Wno-shadow` on the `bumbledb` module target
  (`src/CMakeLists.txt`) because other partitions instantiate the six
  `template for` sites (`closed/axioms.cc`, `closed/facade.cc`,
  `closed/where.cc`, `relation/row.cc`, `query/rule.cc`,
  `schema/classes.cc`); on every runtime and cookbook test target; on
  every compile-fail case (so a case's only diagnostic is the one under
  test); and source-scoped on `tests/conformance.cc`. A source-level
  `COMPILE_OPTIONS` property is used where a single TU instantiates, so
  it lands after the language profile's `-Wshadow` on the command line.
- workaround: `-Wno-shadow` only for the TUs that instantiate expansion
  statements; `-Wshadow` stays on everywhere else. Never an inline
  suppression.
- retire: delete the scoped suppressions when the fix for GCC PR 124197
  ships in the pinned toolchain; re-test on every toolchain bump
- upstream: [GCC PR 124197](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=124197)

## ubsan-constexpr-string

- symptom: `std::string`'s (pointer, size) constructor carries a null
  check that does not constant-fold against ASan-instrumented storage
  (template parameter objects, string literals, `define_static_string`
  globals) **and**, on GCC 17 / trunk, against `name_text` / reflected
  views used as `static_assert` messages — consteval fails as "not a
  constant expression". The iterator-pair constructor folds.
- sites: `bdb::detail::spec_name` (`src/relation/name.cc`) — the single
  funnel, built with `std::string(text.begin(), text.end())`. Every
  injected `data_member_spec` name and every consteval diagnostic string
  built from a `name_text` or `string_view` routes through it (callers in
  `src/closed/facade.cc`, `src/schema/key.cc`, `src/schema/schema.cc`,
  `src/schema/classes.cc`, `src/schema/member.cc`, `src/query/query.cc`,
  `src/query/pattern.cc`, `src/relation/classify.cc`, `src/closed/handle.cc`);
  names straight from `identifier_of` still go through the funnel when
  they become a `std::string` in a consteval message.
- workaround: construct the payload through the iterator-pair
  constructor, and keep every synthesized name and diagnostic string
  routed through the one funnel.
- retire: on any GCC bump, respell `spec_name`'s body as the
  (pointer, size) construction, run the asan-ubsan preset, and re-run
  the compile-fail suite.
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
- retire: delete the per-target check-disable when the pinned
  clang-tidy contains commit `ce6a3d9`. Re-test on any bump: the
  exported cases must be silent, and a plain unused `using ::f;` in the
  purview (the true-positive control) must still warn.
- upstream: nothing to file — this is already fixed. Exact dup
  llvm/llvm-project#162619 (closed-completed), fixed on main by PR
  #183638 ("Teach misc-unused-using-decls that exported using-decls
  aren't unused"), commit `ce6a3d98cc3e`, merged 2026-02-28; trunk
  clang-tidy-24 nightly verified clean 2026-08-09 with the
  true-positive control still warning.

## compile-fail-vs-sanitizers

- symptom: `-fsanitize=address,undefined` breaks constexpr
  `std::string` evaluation inside the reflector's `static_assert`
  message machinery ("is not a constant expression" instead of the
  pinned diagnostic), so instrumented graphs cannot reproduce the
  pinned compile-fail text. Same root family as ubsan-constexpr-string.
- sites: `tests/CMakeLists.txt` — `compile_fail/` is added only when
  `CMAKE_CXX_FLAGS` carries no `-fsanitize` (and never in the lint
  graph, which does not drive the production compiler). Trap-mode UBSan
  lives on `bumbledb_language_profile`, not `CMAKE_CXX_FLAGS`, so
  compile-fail still runs on the dev and release presets.
- workaround: pinned diagnostics are a product of the canonical
  production configurations (dev/release), where the suite runs; CI
  keeps compile-fail as its own uninstrumented job.
- retire: on any GCC bump, configure an instrumented graph with the
  suite included and check the pinned diagnostics reproduce.
- upstream: GCC PR 71962 (the constant-folding root); the exclusion is
  the suite-level face of it.

## gcc-gmf-stdexec-ice

- symptom: cc1plus segfaults whenever `stdexec/execution.hpp` is
  textually included in ANY module unit — interface partition, primary,
  or implementation unit, with or without `import std`. Root cause is a
  global-module-fragment variable declared `extern T const x;` and then
  defined `inline constexpr T x{};` — the stdexec CPO pattern.
- sites: none in this tree. The SDK is a synchronous C ABI over the
  Rust engine and does not vendor stdexec. Recorded so a future
  sender-shaped surface does not FetchContent NVIDIA stdexec into a
  module unit.
- workaround: do not vendor stdexec. If a sender surface appears, every
  stdexec include stays in one plain (non-module) quarantine TU;
  dialect code never includes a stdexec header or spells `stdexec::` /
  `exec::`.
- retire: when libstdc++ ships `__cpp_lib_senders`, bind to
  `std::execution` and drop any vendor pin.
- upstream: [GCC PR 126783](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=126783)
  (`ASSIGNED`, Patrick Palka, milestone 16.3).

## clang-module-stack-protector

- symptom: Clang 22.1 refuses to load the synthesized `std` PCM when the
  importer's stack-protector (and sibling codegen) mode differs from the
  PCM (`clang-diagnostic-module-file-config-mismatch`: "stack protector
  mode differs in precompiled file vs. current file")
- sites: CMakeLists.txt — `-fstack-protector-strong`,
  `-ftrivial-auto-var-init=zero`, and `-fzero-call-used-regs=used-gpr`
  are appended to `CMAKE_CXX_FLAGS` with the other module-ABI flags, not
  to `bumbledb_language_profile`
- workaround: any flag Clang records as a BMI config bit must be global
  so the synthesized import-std target matches every importer
- retire: when the pinned Clang's PCM config no longer includes these
  flags, they can return to the language-profile target; re-test the
  lint graph on every Clang bump
- upstream: none filed — documented Clang module BMI config interacting
  with CMake's synthesized `import std` target

## cmake-ld-link-order

- symptom: the P2900 contracts runtime (`handle_contract_violation`) lives
  in libstdc++exp, and a `-lstdc++exp` spelled in `CMAKE_EXE_LINKER_FLAGS`
  satisfies nothing on Linux: GNU ld resolves left-to-right and the linker
  flags precede the object files on the link line (macOS ld64 masked the
  bug)
- sites: CMakeLists.txt —
  `target_link_libraries(bumbledb_language_profile INTERFACE stdc++exp)`:
  interface-library linkage lands the library AFTER every consumer's
  object files
- workaround: link the contracts runtime through the interface target,
  never through global linker flags
- retire: when the pinned toolchain folds the contracts runtime into
  default libstdc++ linkage (no explicit `stdc++exp` needed); re-check on
  every toolchain bump
- upstream: none — documented GNU ld semantics, nothing to file

## cmake-import-std-uuid

- symptom: `import std` is experimental in CMake, gated by
  `CMAKE_EXPERIMENTAL_CXX_IMPORT_STD`, and the accepted UUID value changes
  per CMake feature series — a stale UUID silently disables the feature
- sites: CMakeLists.txt — the hard configure gate on the pinned CMake series
  plus `set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD
  "d0edc3af-4c50-42ea-a356-e2862fe7a444")` (the pinned-series value)
- workaround: pin the CMake series and the UUID together; on any CMake
  bump re-read `Help/dev/experimental.rst`, update the UUID, and move the
  version gate to the new series
- retire: when CMake ships `import std` as a stable (non-experimental)
  feature and the UUID gate disappears
- upstream: none — CMake's deliberate experimental-feature mechanism
  (`Help/dev/experimental.rst`), nothing to file

## macos-rsize-t-fixinclude

- symptom: the macOS SDK's `sys/_types/_rsize_t.h` assumes
  `__has_feature(modules)` implies clang and uses a clang-only `stddef.h`
  protocol, so under GCC `-fmodules` `rsize_t` never defines and the
  libstdc++ `std` module fails to compile — and libstdc++ silently
  installs a 1-byte `bits/std.cc` fallback (plus its modules.json entry)
  with exit 0
- sites: toolchain acquisition, not repository code — drop a plain-typedef
  copy of `_rsize_t.h` into GCC's `include-fixed/sys/_types/` and rebuild
  libstdc++; verify `include/c++/<ver>/bits/std.cc` is ~113 KB, not 1 byte
- workaround: that local header is toolchain state, not an in-tree file
- retire: when `__need_rsize_t` support lands in the pinned GCC (and the
  silent-empty-fallback report is resolved, so a failed std-module build
  can no longer masquerade as success)
- upstream: filed 2026-08-10 as PR target/126782 (See Also PR 116827); the
  silent empty-module fallback is filed separately as PR 126786.
  Maintainer feedback rejects Darwin fixincludes.

## gcc-darwin-lto-debug-dsymutil

- symptom: any `-flto -g` link on the pinned Darwin toolchain emits objects
  with invalid `__DWARF` sections (dangling DIE references, out-of-bounds
  `DW_AT_stmt_list`); archive members bypass the plugin-less LTO recompile,
  and the driver-run Apple dsymutil then grows without bound on the debug
  map (67 GB RSS in 13 s; one unguarded run kernel-panicked the host)
- sites: CMakeLists.txt — `check_ipo_supported` plus
  `CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON`: whole-program
  optimization is Release-only, and Release carries no `-g`, so no
  configuration links LTO objects while dsymutil runs
- workaround: never combine `-flto` with `-g` on this target; unsupported
  IPO is a configure failure, not a silent degrade
- retire: when the pinned GCC emits `__DWARF` sections that pass
  `dwarfdump --verify` under `-flto -g`, and dsymutil survives the full
  dev-preset IPO link under a memory guard
- upstream: [GCC PR 82005](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=82005)
  and [LLVM issue 102965](https://github.com/llvm/llvm-project/issues/102965).
  Do not file a new GCC report.

## gcc-darwin-fhardened

- symptom: `-fhardened` on aarch64-apple-darwin24 warns `'-fhardened' not
  supported for this target` with warning class 0 — under the profile's
  `-Werror` this is a hard error that `-Wno-hardened` and
  `-Wno-error=hardened` cannot demote — and the umbrella half-applies
  anyway (stack-protector-strong and trivial-auto-var-init=zero engage;
  stack-clash, `_FORTIFY_SOURCE`, and `_GLIBCXX_ASSERTIONS` are silently
  dropped with no `-Whardened` report)
- sites: CMakeLists.txt — hardened codegen is split: stack-protector,
  trivial-auto-var-init, and zero-call-used-regs are global module-ABI
  flags (PIN `clang-module-stack-protector`); GNU-scoped
  `-fstack-clash-protection` stays on `bumbledb_language_profile`
- workaround: never spell `-fhardened`; enable the working constituents
  individually (`-ftrivial-auto-var-init=zero`, `-fstack-protector-strong`,
  `-fzero-call-used-regs=used-gpr`, GNU-scoped `-fstack-clash-protection`)
- retire: replace the individual flags with `-fhardened` when the pinned GCC
  both classifies the unsupported-target warning under `-Whardened` and
  enables the umbrella (or accurate per-constituent reporting) on Darwin;
  re-test the `-E -dM` macro set on every toolchain bump
- upstream: [GCC PR 126822](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=126822)
  (`NEW`). [PR 126823](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=126823) is
  closed locally as designed.

## cmake-ipo-probe-ordering

- symptom: CMake's `check_ipo_supported` probe project inherits
  `CMAKE_CXX_FLAGS` but not `CMAKE_CXX_STANDARD`, and the pinned cc1plus
  rejects `-freflection` outside `-std=c++26`/`-std=gnu++26` — so an IPO
  check placed after the `-freflection` append reports the toolchain
  unsupported and the configure gate FATAL_ERRORs on a working compiler
- sites: CMakeLists.txt — `check_ipo_supported(LANGUAGES CXX)` deliberately
  precedes the `-freflection` `CMAKE_CXX_FLAGS` append (the in-tree comment
  states the constraint)
- workaround: keep the IPO probe ahead of every flag that is valid only at
  the project's language-standard level
- retire: when the pinned CMake's IPO probe honors `CMAKE_CXX_STANDARD` (or
  the pinned GCC accepts `-freflection` at any standard level); re-test by
  reordering the probe on every CMake or GCC bump
- upstream: none filed — documented CMake probe semantics interacting with a
  standard-gated GCC flag; no reduction produced
