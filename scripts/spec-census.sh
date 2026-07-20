#!/usr/bin/env bash
# The spec census (the covenant campaign, PRD 10): the grep-checked half
# of the Bridge. The Lean half of every ledger row is checked by the
# build (each row carries a term-level `@theoremName` reference); this
# script checks the other half:
#
#   (a) every `mechanism` token of `lean/Bumbledb/Bridge.lean` greps to
#       an existing path and symbol under crates/;
#   (b) every `instrument` token greps to an existing test fn or
#       conformance case;
#   (c) every `lean/…` citation in docs/architecture/ and
#       docs/cookbook.md resolves to a real file — and, when it names a
#       declaration (`lean/….lean: name`), to a real declaration in it;
#   (d) every backticked `path.rs::symbol` citation in lean/ doc
#       comments resolves: some file under crates/*/src
#       whose path ends with the cited path contains the symbol's
#       final `::`-segment word-bounded. (Line-number citations inside
#       lean doc comments are NOT checked — they drift silently; prefer
#       the symbol form, which this check keeps honest.)
#
# Parse contract (recorded in Bridge.lean's module doc): mechanism and
# instrument strings are semicolon-joined tokens, each either
# `symbol (path)` — the path must exist and the symbol's final
# `::`-segment must grep word-bounded inside it — or a bare
# `crates/…` path (existence). Premise strings carry none of
# `crates/`, `::`, so only mechanism/instrument strings are
# scanned. Exit nonzero on any dangler. Conventions follow check.sh.
set -euo pipefail

cd "$(dirname "$0")/.."

BRIDGE=lean/Bumbledb/Bridge.lean
fail=0

if [ ! -f "$BRIDGE" ]; then
  echo "spec-census: FAIL — $BRIDGE missing" >&2
  exit 1
fi

# ---- (a) + (b): the ledger's mechanism and instrument tokens ---------

scanned=0
# Every double-quoted string literal in the ledger that carries a
# census-scannable token (strings are single-line by construction).
while IFS= read -r str; do
  case "$str" in
    *crates/* | *::*) ;;
    *) continue ;;
  esac
  # Split the string on '; ' into tokens.
  while IFS= read -r tok; do
    [ -n "$tok" ] || continue
    scanned=$((scanned + 1))
    if [[ "$tok" =~ ^(.+)\ \((crates/[^\)]+)\)$ ]]; then
      sym="${BASH_REMATCH[1]}"
      path="${BASH_REMATCH[2]}"
      final="${sym##*::}"
      if [ ! -e "$path" ]; then
        echo "spec-census: FAIL — path '$path' (token '$tok') does not exist" >&2
        fail=1
      elif ! grep -qw -- "$final" "$path"; then
        echo "spec-census: FAIL — symbol '$final' (token '$tok') not found in $path" >&2
        fail=1
      fi
    elif [[ "$tok" =~ ^crates/[A-Za-z0-9_./-]+$ ]]; then
      if [ ! -e "$tok" ]; then
        echo "spec-census: FAIL — path '$tok' does not exist" >&2
        fail=1
      fi
    else
      echo "spec-census: FAIL — unparseable census token '$tok' (the parse contract: 'symbol (path)' or a bare crates/ path)" >&2
      fail=1
    fi
  done < <(printf '%s\n' "$str" | sed 's/; /\n/g')
done < <(grep -o '"[^"]*"' "$BRIDGE" | sed 's/^"//; s/"$//')

if [ "$scanned" -eq 0 ]; then
  echo "spec-census: FAIL — no census tokens found in $BRIDGE (ledger empty or convention drifted)" >&2
  fail=1
fi

# The ledger count: the asserted literal must match the row count, so
# the census notices a drifted assertion even before the build does.
rows=$(grep -c '\.row @' "$BRIDGE")
asserted=$(sed -n 's/.*ledger\.length = \([0-9][0-9]*\).*/\1/p' "$BRIDGE" | head -n 1)
if [ -z "$asserted" ] || [ "$rows" -ne "$asserted" ]; then
  echo "spec-census: FAIL — ledger has $rows rows but asserts ${asserted:-nothing}" >&2
  fail=1
fi

# ---- (c): docs-side lean/ citation integrity --------------------------

docs=(docs/architecture/*.md docs/cookbook.md)

# Bare lean/ path citations: the file (or directory) must exist.
while IFS= read -r cite; do
  cite="${cite%%[),.:\`]}" # strip trailing punctuation the prose adds
  [ -n "$cite" ] || continue
  if [ ! -e "$cite" ]; then
    echo "spec-census: FAIL — docs cite '$cite' which does not exist" >&2
    fail=1
  fi
done < <(grep -ohE 'lean/[A-Za-z0-9_/.-]*' "${docs[@]}" | sort -u)

# Declaration citations `lean/….lean: name`: the declaration's final
# dot-segment must grep word-bounded inside the cited file.
while IFS= read -r cite; do
  file="${cite%%:*}"
  decl="$(printf '%s' "${cite#*:}" | tr -d ' ')"
  final="${decl##*.}"
  if [ ! -f "$file" ]; then
    echo "spec-census: FAIL — docs cite '$file' which does not exist" >&2
    fail=1
  elif ! grep -qw -- "$final" "$file"; then
    echo "spec-census: FAIL — docs cite '$decl' not found in $file" >&2
    fail=1
  fi
done < <(grep -ohE 'lean/[A-Za-z0-9_/.-]+\.lean: *[A-Za-z_][A-Za-z0-9_.]*' "${docs[@]}" | sort -u)

# ---- (d): lean-side rust symbol citations ----------------------------
# The normative spec's doc comments anchor recorded narrowings to rust
# code. Line-number anchors drift silently (the 2026-07-15 fidelity
# review found four drifted ranges); symbol anchors are checkable, so
# they are what this lane keeps honest: `path.rs::symbol` in backticks,
# path resolved as a suffix under crates/*/src, the
# symbol's final `::`-segment grepped word-bounded in a matching file.

lean_cites=0
while IFS= read -r cite; do
  [ -n "$cite" ] || continue
  lean_cites=$((lean_cites + 1))
  path="${cite%%::*}"
  sym="${cite#*::}"
  final="${sym##*::}"
  found=0
  while IFS= read -r cand; do
    if grep -qw -- "$final" "$cand"; then
      found=1
      break
    fi
  done < <([ -f "$path" ] && printf '%s\n' "$path"; \
           find crates/*/src -type f -path "*/$path" 2>/dev/null; \
           find crates/*/src -type f -name "$path" 2>/dev/null)
  if [ "$found" -ne 1 ]; then
    echo "spec-census: FAIL — lean cites '$cite' but no crates/*/src file matching '$path' contains '$final'" >&2
    fail=1
  fi
done < <(grep -rhoIE --include='*.lean' --include='*.md' --exclude-dir=.lake \
           '`[A-Za-z0-9_/.-]+\.rs::[A-Za-z0-9_:]+`' lean/ \
           | sed 's/^`//; s/`$//' | sort -u)

if [ "$lean_cites" -eq 0 ]; then
  echo "spec-census: FAIL — no lean-side symbol citations found (convention drifted?)" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "spec-census: OK — $rows ledger rows, $scanned tokens resolved, docs citations intact, $lean_cites lean symbol citations resolved"
