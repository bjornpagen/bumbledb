#!/usr/bin/env bash
# Sample an already-built program. Build/test/verify BEFORE this command.
# Profiles and symbols stay local; no upload and no browser launch.
set -euo pipefail

if [ "$#" -lt 2 ]; then
    echo "usage: profile.sh <fresh-out-dir> <built-binary> [arguments...]" >&2
    echo "build: cargo build --profile profiling -p bumbledb-bench" >&2
    echo "tool: cargo install --locked samply --version 0.13.1" >&2
    exit 2
fi

REPO="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$1"
BIN="$2"
shift 2
SAMPLY="${BUMBLEDB_SAMPLY:-samply}"
command -v "$SAMPLY" >/dev/null || {
    echo "profile.sh: samply not found (set BUMBLEDB_SAMPLY or install it)" >&2
    exit 2
}
if [ ! -f "$BIN" ] || [ ! -x "$BIN" ]; then
    echo "profile.sh: expected an already-built executable: $BIN" >&2
    exit 2
fi
if [ "$(uname -s)" = Darwin ] && [ ! -d "$BIN.dSYM" ]; then
    echo "profile.sh: missing packed symbols: $BIN.dSYM" >&2
    echo "build the profiling profile; preserve its dSYM with cp -RL when freezing the binary" >&2
    exit 2
fi
if [ -e "$OUT" ]; then
    echo "profile.sh: refusing to overwrite $OUT; choose a fresh directory" >&2
    exit 2
fi
if [ "${BUMBLEDB_PROFILE_UNDER_LOCK:-0}" != 1 ]; then
    exec "$REPO/scripts/measure.sh" env BUMBLEDB_PROFILE_UNDER_LOCK=1 \
        bash "$0" "$OUT" "$BIN" "$@"
fi

mkdir -p "$(dirname "$OUT")"
mkdir "$OUT"
OUT="$(cd "$OUT" && pwd)"
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
FROZEN="$OUT/$(basename "$BIN")"
# The native read driver publishes its report only after a complete window.
# Samply can exit zero after a child signal; its exit status alone is not a
# success gate. Keep generic command captures available, but validate every
# `profile` workload automatically, before announcing a completed capture.
WORKLOAD_REPORT=""
if [ "${1:-}" = profile ]; then
    ARGS=("$@")
    WORKLOAD_OUT=""
    for ((i=1; i<${#ARGS[@]}; i++)); do
        if [ "${ARGS[$i]}" = --out ]; then
            if ((i + 1 >= ${#ARGS[@]})); then
                echo "profile.sh: --out needs a workload directory" >&2
                exit 2
            fi
            WORKLOAD_OUT="${ARGS[$((i + 1))]}"
        fi
    done
    if [ -z "$WORKLOAD_OUT" ]; then
        WORKLOAD_OUT="$OUT/workload"
        set -- "$@" --out "$WORKLOAD_OUT"
    fi
    if [ -e "$WORKLOAD_OUT" ] || [ -L "$WORKLOAD_OUT" ]; then
        echo "profile.sh: refusing existing workload output $WORKLOAD_OUT" >&2
        exit 2
    fi
    WORKLOAD_REPORT="$WORKLOAD_OUT/workload.json"
fi
cp "$BIN" "$FROZEN"
if [ -d "$BIN.dSYM" ]; then
    cp -RL "$BIN.dSYM" "$FROZEN.dSYM"
fi
git -C "$REPO" diff HEAD --binary > "$OUT/source.patch"
while IFS= read -r -d '' path; do
    status=0
    git -C "$REPO" diff --no-index --binary /dev/null "$path" >> "$OUT/source.patch" || status=$?
    if [ "$status" -gt 1 ]; then exit "$status"; fi
done < <(git -C "$REPO" ls-files --others --exclude-standard -z)
printf '%s\0' "$FROZEN" "$@" > "$OUT/command.argv"
{
    date -u '+timestamp: %Y-%m-%dT%H:%M:%SZ'
    git -C "$REPO" rev-parse HEAD
    rustc --version --verbose
    uname -a
    if [ "$(uname -s)" = Darwin ]; then
        sw_vers
        sysctl -n machdep.cpu.brand_string hw.memsize hw.pagesize
    fi
    "$SAMPLY" --version
    shasum -a 256 "$FROZEN" "$OUT/source.patch"
    printf 'input_binary: %s\n' "$BIN"
    printf 'source_context: capture-time worktree relative to the recorded HEAD\n'
    printf 'bench_boost: %s\n' "${BUMBLEDB_BENCH_BOOST:-0}"
    printf 'sampling_hz: 1000\ndiagnostic_only: true\n'
} > "$OUT/provenance.txt"

# Preserve debug files for address-specific inline frames and source locations.
# Samply 0.13.1's unstable presymbolication coalesces debug info by function,
# incorrectly sharing one inline stack across different instruction addresses.
# Do not enable it. `flame.py native` resolves every address independently.
# No function-entry instrumentation, allocator counters, or frame-pointer
# changes are added to the sampled engine.
"$SAMPLY" record --save-only --rate 1000 \
    --symbol-dir "$OUT" --output "$OUT/profile.json.gz" \
    "$FROZEN" "$@" > "$OUT/capture.log" 2>&1
if [ -n "$WORKLOAD_REPORT" ]; then
    python3 "$REPO/scripts/flame.py" check-workload "$OUT/profile.json.gz" "$WORKLOAD_REPORT"
fi
printf 'profile: %s\n' "$OUT/profile.json.gz"
printf 'view locally: samply load %q\n' "$OUT/profile.json.gz"
printf 'export CPU flamegraph: BUMBLEDB_SAMPLY=%q python3 %q native %q %q' \
    "$SAMPLY" "$REPO/scripts/flame.py" "$OUT/profile.json.gz" "$OUT/cpu"
if [ -n "$WORKLOAD_REPORT" ]; then
    printf ' --workload %q' "$WORKLOAD_REPORT"
fi
printf '\n'
