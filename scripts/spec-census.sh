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
#   (e) every backticked Lean declaration name in lean/ markdown
#       (`Txn.judgeB`, …) resolves: its final dot-segment greps
#       word-bounded somewhere in the Lean sources — the same rule (c)
#       applies to the docs' `lean/….lean: name` citations, applied
#       where lean-side prose cites declarations directly (so a
#       renamed theorem cannot live on in a README).
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

# ---- (e): lean-side Lean declaration citations -------------------------
# Backticked dotted declaration names in lean/ markdown. The case filter
# drops file and path spellings (`Bridge.lean`, `cases/foo.json`) — they
# carry no checkable declaration and (c)'s existence check owns paths.

lean_decl_cites=0
while IFS= read -r cite; do
  [ -n "$cite" ] || continue
  case "$cite" in
    */* | *.lean | *.md | *.json | *.rs | *.toml) continue ;;
  esac
  lean_decl_cites=$((lean_decl_cites + 1))
  final="${cite##*.}"
  if ! grep -rqw --include='*.lean' --exclude-dir=.lake -- "$final" lean/; then
    echo "spec-census: FAIL — lean markdown cites '$cite' but '$final' resolves in no Lean source" >&2
    fail=1
  fi
done < <(grep -rhoIE --include='*.md' --exclude-dir=.lake \
           '`[A-Z][A-Za-z0-9_]*(\.[A-Za-z0-9_'\''!?]+)+`' lean/ \
         | sed 's/^`//; s/`$//' | sort -u)

if [ "$lean_decl_cites" -eq 0 ]; then
  echo "spec-census: FAIL — no lean-side declaration citations found (convention drifted?)" >&2
  fail=1
fi

# ---- (f): deleted API-sense snapshot token ---------------------------
# The retired public type `Snapshot` / error `ForeignSnapshot` cannot
# return in the surfaces audit/17 named. API words are `ReadInstance`
# and `Witness`. Not this token:
#   * Lean `structure Snapshot` (mathematical consistent-state premise)
#   * README / 00-product "MVCC snapshots" (LMDB semantics)
#   * "reader snapshot isolation", "restored snapshots" (backups),
#     concurrent LMDB reader slots, the parked-lease generation rule
#     ("inside its own snapshot", "open-snapshot/read-counter")
#   * Fact/Key rustdoc borrowing "the snapshot's dictionary" (LMDB CoW)

deleted_spelling='ForeignSnapshot|`Snapshot`|Snapshot<'\''|&Snapshot([^A-Za-z_]|$)|crate::Snapshot|bumbledb::Snapshot'

api_snapshot_docs=(
  docs/cookbook.md
  docs/architecture/00-product.md
  docs/architecture/50-storage.md
  docs/architecture/10-data-model.md
)
api_snapshot_docs_allow='MVCC snapshot|MVCC read snapshot|reader snapshot isolation|restored snapshot|WAL/snapshot|concurrent snapshot|the snapshot past the table|inside its own snapshot|open-snapshot'

api_snapshot_rustdoc=(
  crates/bumbledb/src/api/db.rs
  crates/bumbledb/src/api/db/prepare.rs
  crates/bumbledb/src/api/db/read_instance.rs
  crates/bumbledb/src/api/db/get.rs
)
api_snapshot_rustdoc_allow='LMDB snapshots|the snapshot'\''s committed|the snapshot'\''s dictionary'

# Deleted type/error spelling on the named surfaces. Txn.lean is
# scanned for `ForeignSnapshot` only — Lean `structure Snapshot` stays.
while IFS= read -r hit; do
  echo "spec-census: FAIL — deleted API-sense snapshot token: $hit" >&2
  fail=1
done < <(grep -nE -- "$deleted_spelling" \
  "${api_snapshot_docs[@]}" \
  "${api_snapshot_rustdoc[@]}" \
  lean/conformance/README.md \
  2>/dev/null || true)
while IFS= read -r hit; do
  echo "spec-census: FAIL — deleted API-sense snapshot token: $hit" >&2
  fail=1
done < <(grep -nE -- 'ForeignSnapshot' lean/Bumbledb/Txn.lean || true)

# Lowercase/API-handle "snapshot" in the named docs: only the allowlisted
# LMDB/backup/WAL homonyms may remain.
while IFS= read -r hit; do
  text="${hit#*:}"
  text="${text#*:}"
  if ! printf '%s' "$text" | grep -qE -- "$api_snapshot_docs_allow"; then
    echo "spec-census: FAIL — API-sense snapshot token: $hit" >&2
    fail=1
  fi
done < <(grep -nE -- '\bsnapshots?\b' "${api_snapshot_docs[@]}" || true)

# Same gate on the named rustdoc files: only LMDB/CoW dictionary wording.
while IFS= read -r hit; do
  text="${hit#*:}"
  text="${text#*:}"
  if ! printf '%s' "$text" | grep -qE -- "$api_snapshot_rustdoc_allow"; then
    echo "spec-census: FAIL — API-sense snapshot token: $hit" >&2
    fail=1
  fi
done < <(grep -nE -- '\bsnapshots?\b' "${api_snapshot_rustdoc[@]}" || true)

# ---- (g): zero-dyn engine (audit/27) --------------------------------
# Production src of the engine crates. `dyn` is legal only on
# `Error::source` and the `ErrorDescriptor` mirror that feeds Display.

engine_src=(
  crates/bumbledb/src
  crates/bumbledb-theory/src
  crates/bumbledb-query/src
  crates/bumbledb-macros/src
)

dyn_exempt_hits=0
while IFS= read -r path; do
  [ -n "$path" ] || continue
  lineno=0
  while IFS= read -r line || [ -n "$line" ]; do
    lineno=$((lineno + 1))
    trimmed="${line#"${line%%[![:space:]]*}"}"
    case "$trimmed" in
      //*) continue ;;
    esac
    code=$(awk '
      {
        out = ""
        in_str = 0
        n = length($0)
        for (i = 1; i <= n; i++) {
          c = substr($0, i, 1)
          if (in_str) {
            if (c == "\\") { i++; continue }
            if (c == "\"") in_str = 0
          } else if (c == "\"") {
            in_str = 1
          } else {
            out = out c
          }
        }
        print out
      }
    ' <<< "$line")
    code="${code%%//*}"
    case "$code" in
      *dyn[[:space:]]* | *$'\tdyn '*) ;;
      *) continue ;;
    esac
    # Word-boundary `dyn` followed by whitespace (type position).
    if ! printf '%s' "$code" | grep -qE '(^|[^A-Za-z0-9_])dyn[[:space:]]'; then
      continue
    fi
    exempt=0
    if [ "$path" = "crates/bumbledb/src/error.rs" ] && {
         [ "$trimmed" = "pub source: Option<&'a (dyn std::error::Error + 'static)>," ] ||
         [ "$trimmed" = "source: &'a (dyn std::error::Error + 'static)," ]
       }; then
      exempt=1
    elif [ "$path" = "crates/bumbledb/src/error/convert.rs" ] &&
         [ "$trimmed" = "fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {" ]; then
      exempt=1
    fi
    if [ "$exempt" -eq 1 ]; then
      dyn_exempt_hits=$((dyn_exempt_hits + 1))
    else
      echo "spec-census: FAIL — engine dyn outside Error::source exemption: $path:$lineno: $line" >&2
      fail=1
    fi
  done < "$path"
done < <(find "${engine_src[@]}" -type f -name '*.rs' \
  ! -name 'tests.rs' ! -path '*/tests/*' | sort)

if [ "$dyn_exempt_hits" -ne 3 ]; then
  echo "spec-census: FAIL — Error::source exemption drifted (expected 3 lines, found $dyn_exempt_hits)" >&2
  fail=1
fi

# ---- (h): purged store-and-value tokens (audit/40) --------------------
# The public engine is the store and the proven value. These spellings
# cannot return as live API. History / deleted-vocabulary / purged /
# add-back lines may name them. Lean `def Instance` is the mathematical
# map — not this gate.

purged_spelling='Db::ephemeral|bdb_db_ephemeral|StoreKindMismatch|StoreKind::|enum StoreKind|`StoreKind`|\bStoreKind\b|\bephemeral\b|\bexhume\b|Exhumed|ExhumeHandle|DescriptorMissing|DescriptorRoundTrip|META_STORE_KIND|META_SCHEMA_DESCRIPTOR|trait Instance|Instance<S>|sealed Instance|enum EnvMode|`EnvMode`|EnvMode::|\bEnvMode\b|persisted descriptor|self-describing'

purged_docs=(
  docs/architecture/*.md
  docs/cookbook.md
  docs/feature-register.md
  docs/design/*.md
  docs/research/*.md
  ts/PUBLISHING.md
)
purged_allow='purged|add-back|Add-back'

while IFS= read -r hit; do
  text="${hit#*:}"
  text="${text#*:}"
  if ! printf '%s' "$text" | grep -qE -- "$purged_allow"; then
    echo "spec-census: FAIL — purged store-and-value token: $hit" >&2
    fail=1
  fi
done < <(grep -nEi -- "$purged_spelling" "${purged_docs[@]}" 2>/dev/null || true)

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "spec-census: OK — $rows ledger rows, $scanned tokens resolved, docs citations intact, $lean_cites lean symbol citations resolved, $lean_decl_cites lean declaration citations resolved, API-sense snapshot token absent, zero-dyn exemption pinned, purged store-and-value tokens absent outside history"
