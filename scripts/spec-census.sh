#!/usr/bin/env bash
# Compact correspondence census (L19). Resolves current Bridge constructors
# and refuses deleted commit/braid vocabulary as a claimed mechanism.
# Not a wording ban, dyn-count, or log-golden pin. Those useful checks
# are assigned: identity/surface goldens → L08/L21; dyn/wording quotas deleted.
set -euo pipefail

cd "$(dirname "$0")/.."

BRIDGE=lean/Bumbledb/Bridge.lean
CORRESP=lean/correspondence.md
LEDGER=lean/proof-bridge-ledger.md
fail=0

if [ ! -f "$BRIDGE" ]; then
  echo "spec-census: FAIL — $BRIDGE missing" >&2
  exit 1
fi
if [ ! -f "$CORRESP" ]; then
  echo "spec-census: FAIL — $CORRESP missing (correspondence catalog)" >&2
  exit 1
fi

# ---- Bridge tokens resolve to a live path and symbol --------------------
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
    elif [[ "$tok" =~ ^lean/[A-Za-z0-9_./-]+$ ]]; then
      if [ ! -e "$tok" ]; then
        echo "spec-census: FAIL — path '$tok' does not exist" >&2
        fail=1
      fi
    else
      echo "spec-census: FAIL — unparseable correspondence token '$tok'" >&2
      fail=1
    fi
  done < <(printf '%s\n' "$str" | sed 's/; /\n/g')
done < <(grep -o '"[^"]*"' "$BRIDGE" | sed 's/^"//; s/"$//')

if [ "$scanned" -eq 0 ]; then
  echo "spec-census: FAIL — no correspondence tokens in $BRIDGE" >&2
  fail=1
fi

rows=$(grep -c '\.row @' "$BRIDGE")
asserted=$(sed -n 's/.*ledger\.length = \([0-9][0-9]*\).*/\1/p' "$BRIDGE" | head -n 1)
if [ -z "$asserted" ] || [ "$rows" -ne "$asserted" ]; then
  echo "spec-census: FAIL — ledger has $rows rows but asserts ${asserted:-nothing}" >&2
  fail=1
fi

# ---- Current constructors must be the claimed mechanisms ----------------
for required in \
  'judge_complete (crates/bumbledb/src/schema/judge.rs)' \
  'judge_incremental (crates/bumbledb/src/schema/judge.rs)' \
  'judge_final_state (crates/bumbledb/src/schema/judge.rs)' \
  'LawfulParent (crates/bumbledb/src/schema/judge.rs)' \
  'ChangeSet (crates/bumbledb/src/changes.rs)'
do
  if ! grep -Fq -- "$required" "$BRIDGE"; then
    echo "spec-census: FAIL — Bridge omits current constructor token: $required" >&2
    fail=1
  fi
done

# ---- Negative: deleted vocabulary cannot be a current mechanism ---------
while IFS= read -r hit; do
  [ -n "$hit" ] || continue
  if printf '%s' "$hit" | grep -qiE 'retired|deleted|cannot certify|not a current|not mechanisms'; then
    continue
  fi
  echo "spec-census: FAIL — deleted commit/braid vocabulary claimed as current: $hit" >&2
  fail=1
done < <(grep -nE -- 'storage/commit|storage/delta\.rs|WriteDelta|Txn/Braids|ComponentClosed' "$BRIDGE" || true)

# ---- lean/ markdown citations resolve ----------------------------------
docs=(lean/README.md lean/conformance/README.md lean/correspondence.md lean/proof-bridge-ledger.md)
for f in "${docs[@]}"; do
  if [ ! -f "$f" ]; then
    echo "spec-census: FAIL — missing '$f'" >&2
    fail=1
  fi
done

while IFS= read -r cite; do
  cite="${cite%%[),:\`]}"
  [ -n "$cite" ] || continue
  case "$cite" in
    lean/conformance/cases|lean/conformance/cases/*) continue ;;
  esac
  if [ ! -e "$cite" ]; then
    echo "spec-census: FAIL — docs cite '$cite' which does not exist" >&2
    fail=1
  fi
done < <(grep -ohE 'lean/[A-Za-z0-9_/.-]+' "${docs[@]}" | sort -u)

# ---- Correspondence catalog names the required cases --------------------
for case_id in \
  C-D26-collision-empty-delta \
  C-D26-unready-cannot-mint \
  C-D04-collision-bytes \
  C-D04-citations-topk \
  C-D05-remint-spill \
  C-D19-cancel \
  C-G04-error-surfaces \
  C-G07-authority
do
  if ! grep -Fq -- "$case_id" "$CORRESP"; then
    echo "spec-census: FAIL — correspondence catalog missing $case_id" >&2
    fail=1
  fi
done

if ! grep -Fq 'history_model.rs' "$CORRESP" "$LEDGER"; then
  echo "spec-census: FAIL — authority-order correspondence must name the independent history model" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "spec-census: OK — $rows ledger rows, $scanned constructor tokens resolved, correspondence catalog intact"
