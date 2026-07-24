#!/usr/bin/env bash
# The Lean gate (the covenant campaign, PRD 01; census added by PRD 10):
# build the spec tree, run the placeholder battery, then run the spec
# census (scripts/spec-census.sh — the Bridge's grep-checked half).
# Exit nonzero on any failure. One entry point: CI's lean job runs
# exactly this script.
#
# The battery's shaping (recorded per PRD 01):
#   * `sorry` and `admit` are term-level proof escapes that can appear
#     anywhere in a line, so they are matched as whole words ANYWHERE
#     in the tree — comments, docstrings, and lean/README.md included.
#     That deliberately over-bans: prose in lean/ must paraphrase
#     ("proof escape", "placeholder") rather than spell the tokens.
#     Strictness errs safe; no comment-parsing, no false negatives.
#   * `axiom` is matched only as a line-leading declaration keyword
#     (optionally after declaration modifiers), because the word
#     legitimately appears in prose ("ground axioms", "the scale
#     axiom"). A doc line must simply never START with the keyword —
#     cheaper and stricter than parsing block comments.
#   * No `partial`-escape check: partial defs are allowed only in the
#     PRD 13 conformance driver, per that PRD (noted in PRD 01 §3).
set -euo pipefail

cd "$(dirname "$0")/../lean"

lake build

fail=0

# Battery 1: proof-escape tokens, whole-word, anywhere in the tree
# (excluding lake's build/manifest machinery, which we do not author).
if grep -rnE --include='*.lean' --include='*.md' --include='*.toml' \
    --exclude-dir='.lake' \
    '(^|[^[:alnum:]_])(sorry|admit)([^[:alnum:]_]|$)' . ; then
  echo "lean.sh: FAIL — proof-escape token found (battery 1)" >&2
  fail=1
fi

# Battery 2: `axiom` as a declaration keyword in Lean sources.
if grep -rnE --include='*.lean' --exclude-dir='.lake' \
    '^[[:space:]]*((private|protected|noncomputable|unsafe|scoped|local)[[:space:]]+)*axiom[[:space:]]' . ; then
  echo "lean.sh: FAIL — axiom declaration found (battery 2)" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

# Battery 3 (PRD 10): the spec census — the Bridge ledger's mechanism
# and instrument tokens resolve against crates/, and the
# docs' lean/ citations resolve against the tree.
cd ..
scripts/spec-census.sh

# Battery 4 (PRD 13): the conformance corpus run — the executable
# denotation (`lake exe conformance`, built by the `lake build` above:
# the exe is a default target) evaluates every checked-in case and
# compares against the recorded engine answers. The driver enumerates
# the directory and prints the live case count in its own summary line
# (lean/Main.lean), so no count is pinned here; seconds-scale on the
# pinned M2 Max — comfortably per-push.
cd lean
lake exe conformance conformance/cases
cd ..

# Battery 5: the full three-way comparator (engine + naive + Lean) —
# the `#[ignore]`d cargo test that replays the corpus through the real
# engine and the naive model, byte-holds the files, and re-runs the
# Lean denotation over the same cases. It lives here, not in check.sh:
# the Lean-dependent lane owns the Lean-dependent test, so check.sh
# stays toolchain-independent and the third oracle still gates every
# lean.sh run (~14 s measured on the pinned M2 Max).
three_way_log=$(cargo test -p bumbledb-bench --lib \
  -- --ignored --exact conformance::tests::three_way_conformance_over_the_checked_in_corpus 2>&1) || {
  printf '%s\n' "$three_way_log" >&2
  echo "lean.sh: FAIL — the three-way comparator reddened (battery 5)" >&2
  exit 1
}
printf '%s\n' "$three_way_log"
# `--exact` with a stale name runs zero tests and still exits 0 — refuse
# the vacuous pass so a rename can never silently drop the third oracle.
if ! printf '%s\n' "$three_way_log" | grep -q 'test result: ok. 1 passed'; then
  echo "lean.sh: FAIL — the three-way comparator did not run (battery 5: 1 passed expected)" >&2
  exit 1
fi

echo "lean.sh: OK — build green, placeholder battery clean, census resolved, conformance corpus green, three-way comparator green"
