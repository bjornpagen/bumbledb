# meta/

Toolchain quarantine, not an architectural layer (TODO_CPP §4, §32). Code
lands here for exactly one reason: it uses C++26 reflection syntax
(reflection expressions, splicing, `std::meta`, expansion statements,
annotations) that the pinned Clang frontend cannot parse yet, so it must be
excluded from the lint graph. The code inside is ordinary `bdb::`
functionality; every AGENTS.md rule applies — the only concession is that
these translation units are checked by GCC diagnostics plus code review
instead of clang-tidy. When Clang learns to parse reflection, the contents
move back next to their callers and this directory disappears.

Module units live here as `.cppm`/`.cpp`, explicitly listed in a
`FILE_SET CXX_MODULES` — never globbed. The top-level `CMakeLists.txt`
excludes this directory only from the lint graph.

Known GCC 16.1 quirk (TODO_CPP §32): expansion statements (`template for`)
re-declare the loop variable per iteration in a nested scope, tripping
`-Wshadow` when the expanded range has more than one element; affected
module sets carry a scoped `-Wno-shadow` in their CMakeLists with this quirk
pinned in a comment — never a per-line suppression.
