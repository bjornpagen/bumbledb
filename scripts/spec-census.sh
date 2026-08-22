#!/usr/bin/env bash
# (a) every `mechanism` token of `lean/Bumbledb/Bridge.lean` greps to
# (c) every `lean/…` citation in the surviving markdown (lean/README.md,
# lean/conformance/README.md, docs/cookbook.md, ts/COOKBOOK.md,
set -euo pipefail

cd "$(dirname "$0")/.."

BRIDGE=lean/Bumbledb/Bridge.lean
fail=0

if [ ! -f "$BRIDGE" ]; then
  echo "spec-census: FAIL — $BRIDGE missing" >&2
  exit 1
fi

scanned=0

while IFS= read -r str; do
  case "$str" in
    *crates/* | *::*) ;;
    *) continue ;;
  esac

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

rows=$(grep -c '\.row @' "$BRIDGE")
asserted=$(sed -n 's/.*ledger\.length = \([0-9][0-9]*\).*/\1/p' "$BRIDGE" | head -n 1)
if [ -z "$asserted" ] || [ "$rows" -ne "$asserted" ]; then
  echo "spec-census: FAIL — ledger has $rows rows but asserts ${asserted:-nothing}" >&2
  fail=1
fi

docs=(lean/README.md lean/conformance/README.md docs/cookbook.md ts/COOKBOOK.md proposals/README.md RULINGS.md REPRESENTATION-FIRST.md)
if [ "${#docs[@]}" -eq 0 ]; then
  echo "spec-census: FAIL — lane (c) scanned zero markdown files (vacuous pass)" >&2
  fail=1
fi
for f in "${docs[@]}"; do
  if [ ! -f "$f" ]; then
    echo "spec-census: FAIL — lane (c) missing '$f' (vacuous pass)" >&2
    fail=1
  fi
done

while IFS= read -r cite; do
  cite="${cite%%[),:\`]}" 
  [ -n "$cite" ] || continue
  if [ ! -e "$cite" ]; then
    echo "spec-census: FAIL — docs cite '$cite' which does not exist" >&2
    fail=1
  fi
done < <(grep -ohE 'lean/[A-Za-z0-9_/.-]*' "${docs[@]}" | sort -u)

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
  README.md
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
  docs/cookbook.md
  ts/PUBLISHING.md
  README.md
  RULINGS.md
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

# ---- (i): comment hygiene (proposals/purge/comment-gates.md) ----------
# Banned tokens in comments across crates/*/src, ts/src, ts/crate, lean/.
# Allowlist mechanism mirrors lane (f): exact-phrase entries, each with a
# one-line justification. Zero-match assertion per token; a nonzero match
# names file:line and fails the lane. Loud vacuous guard: scanning zero
# comments is a fail, not a pass.
#
# DISABLED until wave 4 (the comment purge must land first). Enable with
# BUMBLEDB_CENSUS_LANE_I=1.
if [ "${BUMBLEDB_CENSUS_LANE_I:-}" = "1" ]; then
  lane_i_scanned=0
  lane_i_allow=''
  # Justification roster (exact phrases; empty until wave-4 enablement
  # records the licensed survivors that still spell a banned token):
  #   (none yet — wave 4 fills this when the purge is done)

  banned_tokens=(
    'audit/'
    'PRD '
    'docs/architecture'
    'formerly'
    'previously'
    ' was deleted'
    'no longer'
    'TODO'
    'FIXME'
    'XXX'
  )
  hash_re='[0-9a-f]{8,10}'

  extract_comments() {
    scripts/comment-diff-guard.sh --extract-comments "$1" 2>/dev/null || true
  }

  while IFS= read -r src; do
    [ -n "$src" ] || continue
    case "$src" in
      *'/target/'* | *'/node_modules/'* | *'/.lake/'*) continue ;;
    esac
    extracted=$(extract_comments "$src")
    [ -n "$extracted" ] || continue
    while IFS= read -r crow; do
      [ -n "$crow" ] || continue
      lane_i_scanned=$((lane_i_scanned + 1))
      cline="${crow%%:*}"
      ctext="${crow#*:}"
      if [ -n "$lane_i_allow" ] && printf '%s' "$ctext" | grep -qE -- "$lane_i_allow"; then
        continue
      fi
      for tok in "${banned_tokens[@]}"; do
        if printf '%s' "$ctext" | grep -qF -- "$tok"; then
          echo "spec-census: FAIL — comment-hygiene token '$tok': $src:$cline: $ctext" >&2
          fail=1
        fi
      done
      if printf '%s' "$ctext" | grep -qE -- "\\b${hash_re}\\b"; then
        echo "spec-census: FAIL — comment-hygiene commit-hash token: $src:$cline: $ctext" >&2
        fail=1
      fi
    done <<< "$extracted"
  done < <(find crates/*/src ts/src ts/crate lean \
             \( -name '*.rs' -o -name '*.ts' -o -name '*.lean' \) \
             ! -path '*/.lake/*' ! -path '*/target/*' ! -path '*/node_modules/*' \
             | sort)

  if [ "$lane_i_scanned" -eq 0 ]; then
    echo "spec-census: FAIL — lane (i) scanned zero comments (vacuous pass)" >&2
    fail=1
  fi
else
  echo "spec-census: lane (i) comment-hygiene skipped (set BUMBLEDB_CENSUS_LANE_I=1 to enable)"
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "spec-census: OK — $rows ledger rows, $scanned tokens resolved, docs citations intact, $lean_cites lean symbol citations resolved, $lean_decl_cites lean declaration citations resolved, API-sense snapshot token absent, zero-dyn exemption pinned, purged store-and-value tokens absent outside history"
