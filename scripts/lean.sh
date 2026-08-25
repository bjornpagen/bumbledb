#!/usr/bin/env bash
# in the tree — comments, docstrings, and lean/README.md included.
# (optionally after declaration modifiers), because the word
set -euo pipefail

cd "$(dirname "$0")/../lean"

lake build

fail=0

# (excluding lake's build/manifest machinery, which we do not author).
if grep -rnE --include='*.lean' --include='*.md' --include='*.toml' \
    --exclude-dir='.lake' \
    '(^|[^[:alnum:]_])(sorry|admit)([^[:alnum:]_]|$)' . ; then
  echo "lean.sh: FAIL — proof-escape token found (battery 1)" >&2
  fail=1
fi

if grep -rnE --include='*.lean' --exclude-dir='.lake' \
    '^[[:space:]]*((private|protected|noncomputable|unsafe|scoped|local)[[:space:]]+)*axiom[[:space:]]' . ; then
  echo "lean.sh: FAIL — axiom declaration found (battery 2)" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

# (lean/Main.lean), so no count is pinned here; seconds-scale on the
lake exe conformance conformance/cases
cd ..

three_way_log=$(cargo test -p bumbledb-bench --lib \
  -- --ignored --exact conformance::tests::three_way_conformance_over_the_checked_in_corpus 2>&1) || {
  printf '%s\n' "$three_way_log" >&2
  echo "lean.sh: FAIL — the three-way comparator reddened (battery 5)" >&2
  exit 1
}
printf '%s\n' "$three_way_log"

if ! printf '%s\n' "$three_way_log" | grep -q 'test result: ok. 1 passed'; then
  echo "lean.sh: FAIL — the three-way comparator did not run (battery 5: 1 passed expected)" >&2
  exit 1
fi

echo "lean.sh: OK — build green, placeholder battery clean, conformance corpus green, three-way comparator green"
