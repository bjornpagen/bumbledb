#!/usr/bin/env bash
# The Lean gate (the covenant campaign, PRD 01): build the spec tree,
# then run the placeholder battery. Exit nonzero on any failure.
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

echo "lean.sh: OK — build green, placeholder battery clean"
