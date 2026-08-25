#!/usr/bin/env bash
# (a) every `mechanism` token of `lean/Bumbledb/Bridge.lean` greps to
# (c) every `lean/…` citation in the surviving markdown (lean/README.md,
# lean/conformance/README.md, docs/cookbook.md, ts/COOKBOOK.md,
# (k) the banned-token roster (scripts/banned-tokens.txt) is empty of
# hits in crates/bumbledb-log/src, ts-log/src, examples/lambda/src.
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

docs=(lean/README.md lean/conformance/README.md docs/cookbook.md ts/COOKBOOK.md)
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
# The log driver (crates/bumbledb-log/src) is on the roster with one
# pin: caller-owned credential behavior at a foreign async-trait
# boundary; cold path. Any other `dyn` in the log driver fails.

engine_src=(
  crates/bumbledb/src
  crates/bumbledb-theory/src
  crates/bumbledb-query/src
  crates/bumbledb-macros/src
  crates/bumbledb-log/src
)

dyn_error_source_hits=0
dyn_cred_refresh_hits=0
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
      dyn_error_source_hits=$((dyn_error_source_hits + 1))
    elif [ "$path" = "crates/bumbledb/src/error/convert.rs" ] &&
         [ "$trimmed" = "fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {" ]; then
      exempt=1
      dyn_error_source_hits=$((dyn_error_source_hits + 1))
    elif [ "$path" = "crates/bumbledb-log/src/store/s3.rs" ] && {
         [ "$trimmed" = "Refresh(Arc<dyn Fn() -> io::Result<StaticKeys> + Send + Sync>)," ] ||
         [ "$trimmed" = "refresh: Arc<dyn Fn() -> io::Result<StaticKeys> + Send + Sync>," ] ||
         [ "$trimmed" = "Box<dyn std::future::Future<Output = object_store::Result<Arc<AwsCredential>>> + Send + 'a>," ]
       }; then
      # caller-owned credential behavior at a foreign async-trait boundary; cold path
      exempt=1
      dyn_cred_refresh_hits=$((dyn_cred_refresh_hits + 1))
    fi
    if [ "$exempt" -ne 1 ]; then
      echo "spec-census: FAIL — engine dyn outside Error::source exemption: $path:$lineno: $line" >&2
      fail=1
    fi
  done < "$path"
done < <(find "${engine_src[@]}" -type f -name '*.rs' \
  ! -name 'tests.rs' ! -path '*/tests/*' | sort)

if [ "$dyn_error_source_hits" -ne 3 ]; then
  echo "spec-census: FAIL — Error::source exemption drifted (expected 3 lines, found $dyn_error_source_hits)" >&2
  fail=1
fi
if [ "$dyn_cred_refresh_hits" -ne 3 ]; then
  echo "spec-census: FAIL — credential-refresh exemption drifted (expected 3 lines, found $dyn_cred_refresh_hits)" >&2
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
# Always on after the comment purge (wave 4). Allowlist is empty: no
# licensed survivor still spells a banned token.
lane_i_scanned=0
lane_i_allow=''
# Justification roster (exact phrases; empty — the purge left no licensed
# survivors that still spell a banned token):
#   (none)

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
  done < <(find crates/*/src ts/src ts/crate ts-log/src lean \
             \( -name '*.rs' -o -name '*.ts' -o -name '*.lean' \) \
             ! -path '*/.lake/*' ! -path '*/target/*' ! -path '*/node_modules/*' \
             | sort)

if [ "$lane_i_scanned" -eq 0 ]; then
  echo "spec-census: FAIL — lane (i) scanned zero comments (vacuous pass)" >&2
  fail=1
fi

# ---- (j): one-owner protocol constants ---------------------------------
# Every named constant of the log protocol has exactly one defining site
# per language: other files cite the name, never restate the value.

one_owner_rust=(
  LOSS_BOUND
  DRAIN_MAX_WRITES
  DRAIN_MAX_BYTES
  LEASE_WIDTH
  CHECKPOINT_EVERY_SUM
  CHECKPOINT_EVERY_BYTES
  CHECKPOINT_RETAIN_MS
  WAIT_FOR_POLL_MS
  LOCK_RETRY_MS
)
for name in "${one_owner_rust[@]}"; do
  count=$(grep -rE "const ${name}[[:space:]]*:" crates/bumbledb-log/src --include='*.rs' | wc -l | tr -d ' ')
  if [ "$count" -ne 1 ]; then
    echo "spec-census: FAIL — lane (j) constant '$name' has $count defining sites in crates/bumbledb-log/src (one owner required)" >&2
    fail=1
  fi
done

one_owner_ts=(
  LEASE_WIDTH
  LOSS_BOUND
  WAIT_FOR_POLL_MS
  LOCK_RETRY_MS
)
for name in "${one_owner_ts[@]}"; do
  count=$(grep -rE "const ${name}[[:space:]]*=" ts-log/src --include='*.ts' | wc -l | tr -d ' ')
  if [ "$count" -ne 1 ]; then
    echo "spec-census: FAIL — lane (j) constant '$name' has $count defining sites in ts-log/src (one owner required)" >&2
    fail=1
  fi
done

# ---- (k): banned-token roster (50 §1) --------------------------------
# The cutover's absence list is data. Each roster line is a (token, scope)
# pair; a hit prints that line so the violation names its own law.
# Allowlist is what 40/canon already name: hex at inspect / refusal /
# key-grammar / test-metadata; theory-file JSON numbers in schema_file.rs
# (text half); lease decimal ASCII is not quoted-decimal u64.

ROSTER=scripts/banned-tokens.txt
if [ ! -f "$ROSTER" ]; then
  echo "spec-census: FAIL — $ROSTER missing" >&2
  fail=1
fi

roster_lines=0
roster_files=0

roster_allowed() {
  local allow="$1" path="$2" text="$3"
  [ -n "$allow" ] || return 1
  local frag
  IFS=',' read -r -a parts <<< "$allow"
  for frag in "${parts[@]}"; do
    [ -n "$frag" ] || continue
    case "$path" in *"$frag"*) return 0 ;; esac
    case "$text" in *"$frag"*) return 0 ;; esac
  done
  return 1
}

is_protocol_codec() {
  case "$1" in
    crates/bumbledb-log/src/manifest.rs | \
    crates/bumbledb-log/src/sidecar.rs | \
    crates/bumbledb-log/src/codec.rs | \
    crates/bumbledb-log/src/vector.rs | \
    ts-log/src/manifest.ts | \
    ts-log/src/chain.ts | \
    ts-log/src/codec.ts | \
    ts-log/src/bytes.ts | \
    ts-log/src/vector.ts)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

# Hex survives at inspect, refusal text, the key grammar's digest-to-key
# function, and test metadata (40). Definitions and imports of that
# rendering are the same boundary.
is_hex_allow() {
  local path="$1" text="$2"
  case "$path" in
    */inspect.rs | */bin/duty.rs | *tests.rs | *.test.ts) return 0 ;;
  esac
  printf '%s' "$text" | grep -qE '(^|[[:space:]])(pub[[:space:]]+)?(fn|function)[[:space:]]+(hex32|digest32FromHex)\b' && return 0
  printf '%s' "$text" | grep -qE '^[[:space:]]*(use |import |export )' && return 0
  printf '%s' "$text" | grep -qE '^[[:space:]]*(hex32|digest32FromHex),?[[:space:]]*$' && return 0
  printf '%s' "$text" | grep -qE 'ckpt/|ckpt_mdb_key|ckpt_doc_key|checkpointMdbKey|ckptDocKey' && return 0
  printf '%s' "$text" | grep -qE '\b(refuse|refuseManifestMissing|errors\.new|throw)[[:space:](]' && return 0
  printf '%s' "$text" | grep -qE 'kind:|carried:|`' && return 0
  printf '%s' "$text" | grep -qE 'assert!|assert_eq!|assert\.' && return 0
  return 1
}

scan_tsre() {
  python3 - "$1" <<'PY'
import sys

path = sys.argv[1]
src = open(path, encoding="utf-8").read()
n = len(src)
i = 0
line = 1
line_start = 0
last = "op"
hits: list[tuple[int, str]] = []

def line_text(at: int) -> str:
    end = src.find("\n", at)
    if end < 0:
        end = len(src)
    return src[at:end]

def is_word(idx: int, word: str) -> bool:
    if not src.startswith(word, idx):
        return False
    if idx > 0 and (src[idx - 1].isalnum() or src[idx - 1] == "_"):
        return False
    end = idx + len(word)
    if end < n and (src[end].isalnum() or src[end] == "_"):
        return False
    return True

ops = set("=([,!&|?:;{+%^~<>")
while i < n:
    ch = src[i]
    if ch == "\n":
        line += 1
        line_start = i + 1
        i += 1
        continue
    if ch in " \t\r":
        i += 1
        continue
    if src.startswith("//", i):
        i = src.find("\n", i)
        if i < 0:
            break
        continue
    if src.startswith("/*", i):
        end = src.find("*/", i + 2)
        if end < 0:
            break
        chunk = src[i:end]
        line += chunk.count("\n")
        if "\n" in chunk:
            line_start = i + chunk.rfind("\n") + 1
        i = end + 2
        continue
    if ch in "'\"`":
        q = ch
        i += 1
        while i < n:
            if src[i] == "\\":
                i += 2
                continue
            if src[i] == "\n":
                line += 1
                line_start = i + 1
            if q == "`" and src.startswith("${", i):
                i += 2
                depth = 1
                while i < n and depth:
                    if src[i] == "\n":
                        line += 1
                        line_start = i + 1
                    if src[i] == "{":
                        depth += 1
                    elif src[i] == "}":
                        depth -= 1
                    i += 1
                continue
            if src[i] == q:
                i += 1
                break
            i += 1
        last = "val"
        continue
    if is_word(i, "new"):
        j = i + 3
        while j < n and src[j] in " \t":
            j += 1
        if is_word(j, "RegExp"):
            hits.append((line, line_text(line_start)))
            i = j + 6
            last = "val"
            continue
    if ch == "/" and last == "op":
        hits.append((line, line_text(line_start)))
        i += 1
        while i < n and src[i] != "\n":
            if src[i] == "\\":
                i += 2
                continue
            if src[i] == "/":
                i += 1
                while i < n and src[i] in "gimsuyvd":
                    i += 1
                break
            i += 1
        last = "val"
        continue
    if ch.isalpha() or ch == "_" or ch == "$":
        while i < n and (src[i].isalnum() or src[i] in "_$"):
            i += 1
        last = "val"
        continue
    if ch.isdigit():
        while i < n and (src[i].isalnum() or src[i] in "._"):
            i += 1
        last = "val"
        continue
    if ch in ")]":
        last = "val"
        i += 1
        continue
    if ch in ops or ch in "-*/":
        last = "op"
        i += 1
        continue
    i += 1

for lineno, text in hits:
    print(f"{lineno}:{text}")
PY
}

fail_roster() {
  local path="$1" hit="$2" raw="$3"
  echo "spec-census: FAIL — banned token: $path:$hit" >&2
  echo "  roster: $raw" >&2
  fail=1
}

if [ -f "$ROSTER" ]; then
  while IFS= read -r raw || [ -n "$raw" ]; do
    trimmed="${raw#"${raw%%[![:space:]]*}"}"
    case "$trimmed" in
      '' | '#'*) continue ;;
    esac
    roster_lines=$((roster_lines + 1))
    IFS=$'\t' read -r token scope kind needle allow <<< "$trimmed"
    if [ -z "${token:-}" ] || [ -z "${scope:-}" ] || [ -z "${kind:-}" ]; then
      echo "spec-census: FAIL — unparseable roster line: $trimmed" >&2
      fail=1
      continue
    fi
    if [ ! -d "$scope" ]; then
      echo "spec-census: FAIL — roster scope '$scope' does not exist (token '$token')" >&2
      fail=1
      continue
    fi
    case "$kind" in
      fixed | regex | ident | hex | tsre) ;;
      *)
        echo "spec-census: FAIL — roster kind '$kind' is not fixed|regex|ident|hex|tsre: $trimmed" >&2
        fail=1
        continue
        ;;
    esac
    if [ "$kind" != "tsre" ] && [ -z "${needle:-}" ]; then
      echo "spec-census: FAIL — roster needle empty: $trimmed" >&2
      fail=1
      continue
    fi
    scope_hits=0
    while IFS= read -r path; do
      [ -n "$path" ] || continue
      roster_files=$((roster_files + 1))
      scope_hits=$((scope_hits + 1))
      case "$kind" in
        hex)
          if ! is_protocol_codec "$path"; then
            continue
          fi
          ;;
        tsre)
          case "$path" in
            *.ts) ;;
            *) continue ;;
          esac
          ;;
      esac
      hits=""
      case "$kind" in
        fixed)
          hits=$(grep -nF -- "$needle" "$path" || true)
          ;;
        regex)
          hits=$(grep -nE -- "$needle" "$path" || true)
          ;;
        ident)
          hits=$(grep -nE -- "\\b[[:alnum:]]*${needle}[[:alnum:]]*\\b" "$path" || true)
          ;;
        hex)
          hits=$(grep -nE -- "$needle" "$path" || true)
          ;;
        tsre)
          hits=$(scan_tsre "$path" || true)
          ;;
      esac
      [ -n "$hits" ] || continue
      while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        text="${hit#*:}"
        if roster_allowed "${allow:-}" "$path" "$text"; then
          continue
        fi
        if [ "$kind" = "hex" ] && is_hex_allow "$path" "$text"; then
          continue
        fi
        fail_roster "$path" "$hit" "$trimmed"
      done <<< "$hits"
    done < <(find "$scope" -type f \( -name '*.rs' -o -name '*.ts' \) \
      ! -path '*/target/*' ! -path '*/node_modules/*' | sort)
    if [ "$scope_hits" -eq 0 ]; then
      echo "spec-census: FAIL — roster scope '$scope' contains zero source files (token '$token')" >&2
      fail=1
    fi
  done < "$ROSTER"
fi

if [ "$roster_lines" -eq 0 ]; then
  echo "spec-census: FAIL — $ROSTER has zero data lines (vacuous pass)" >&2
  fail=1
fi
if [ "$roster_files" -eq 0 ]; then
  echo "spec-census: FAIL — banned-token roster scanned zero files (vacuous pass)" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "spec-census: OK — $rows ledger rows, $scanned tokens resolved, docs citations intact, $lean_cites lean symbol citations resolved, $lean_decl_cites lean declaration citations resolved, API-sense snapshot token absent, zero-dyn exemption pinned (Error::source 3, credential refresh 3), purged store-and-value tokens absent outside history, one-owner constants single-sited, banned-token roster $roster_lines lines clean"
