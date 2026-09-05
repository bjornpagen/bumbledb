#!/usr/bin/env bash
# Lean kernel + correspondence (qualification). L19 authored this
# rewrite; verification is NotRun during fanout (do not treat a
# worker's unreadiness as Passed).
set -euo pipefail

cd "$(dirname "$0")/../lean"

lake build

fail=0

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

lake exe conformance conformance/cases
cd ..

# Constructor correspondence — current symbols, not dyn/wording quotas.
scripts/spec-census.sh

# Product three-way / bench oracles are L20/L21 qualification, not a
# Lean proof. Do not cargo-test from this script.

echo "lean.sh: OK — build green, placeholder battery clean, conformance corpus green, correspondence census green"
