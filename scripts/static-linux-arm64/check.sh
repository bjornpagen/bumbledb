#!/usr/bin/env bash
# Complete native musl build/qualification; no benchmarks and no skipped phases.
set -euo pipefail
cd "$(dirname "$0")/../.."
repo="$PWD"
target=aarch64-unknown-linux-musl
[[ "$(uname -s)-$(uname -m)" == Linux-aarch64 ]]
[[ "$(cc -dumpmachine)" == aarch64*musl* ]]
[[ -z "${RUSTFLAGS:-}" && -z "${CARGO_ENCODED_RUSTFLAGS:-}" ]] || {
    echo "global Rust flags are not accepted by the generic static qualification" >&2
    exit 2
}
export RUSTUP_TOOLCHAIN
RUSTUP_TOOLCHAIN="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)"
[[ -n "$RUSTUP_TOOLCHAIN" ]]
export CARGO_BUILD_TARGET="$target"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS='-C target-feature=+crt-static'
export CC_aarch64_unknown_linux_musl=cc
export AR_aarch64_unknown_linux_musl=ar
export PYTHONDONTWRITEBYTECODE=1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}"
export CARGO_PROFILE_DEV_DEBUG=1
output="${1:-$CARGO_TARGET_DIR/static-linux-arm64}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
# A failed rerun must not leave an earlier success seal looking current.
rm -f "$output/verification.json" "$output/SHA256SUMS"

echo "==> pinned static toolchain and verifier regressions"
rustc --version
cargo nextest --version
python3 scripts/static-linux-arm64/test_verify_elf.py

echo "==> build every workspace executable, with full default dependencies"
cargo build --locked --workspace --bins --release --target "$target" --message-format=json > "$output/bin-artifacts.jsonl"
python3 scripts/static-linux-arm64/verify-elf.py --cargo-json "$output/bin-artifacts.jsonl" > "$output/workspace-binaries.json"

echo "==> real static core archive, C linker, native runtime and negative controls"
cargo rustc --locked -p bumbledb --release --target "$target" --crate-type rlib,staticlib --message-format=json -- --print native-static-libs > "$output/core-artifacts.jsonl"
python3 scripts/static-linux-arm64/link-probe.py "$output/core-artifacts.jsonl" "$output"
cp "$CARGO_TARGET_DIR/$target/release/duty" "$output/bumbledb-log-duty"
python3 scripts/static-linux-arm64/verify-elf.py "$output/bumbledb-static-smoke" "$output/bumbledb-log-duty" > "$output/static-executables.json"
"$output/bumbledb-static-smoke"
cc scripts/static-linux-arm64/dynamic-negative.c -o "$output/dynamic-negative"
if python3 scripts/static-linux-arm64/verify-elf.py "$output/dynamic-negative" > "$output/dynamic-negative.log" 2>&1; then
    echo "verifier accepted a dynamically linked negative control" >&2
    exit 1
fi
grep -Eq 'PT_INTERP|DT_NEEDED' "$output/dynamic-negative.log"

echo "==> empty-root execution and full-system ARM64 Linux under QEMU"
bash scripts/static-linux-arm64/prepare-rootfs.sh "$output"
python3 scripts/static-linux-arm64/run-qemu.py "$output"

echo "==> all-feature Rust correctness on native ARM64 musl"
cargo nextest run --locked --workspace --all-features --target "$target" --profile ci 2>&1 | tee "$output/musl-tests.log"
cargo test --locked --workspace --all-features --doc --target "$target" 2>&1 | tee "$output/musl-doctests.log"
cargo check --locked -p bumbledb-log --no-default-features --target "$target"
cargo nextest run --locked -p bumbledb --features alloc-counter --test alloc_gate --release --target "$target" --profile ci 2>&1 | tee "$output/musl-allocation-gate.log"

python3 scripts/static-linux-arm64/notices.py "$output"

python3 - "$output" <<'PY'
import datetime
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib
out = Path(sys.argv[1])
report = {
    "verifiedAt": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "sourceRevision": os.environ.get("BUMBLEDB_STATIC_REVISION", "working-tree"),
    "version": tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"],
    "target": "aarch64-unknown-linux-musl",
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    "compiler": subprocess.check_output(["cc", "--version"], text=True).splitlines()[0],
    "checks": ["all workspace bins static", "bundled Rust/C/LMDB/musl/unwinder archive",
               "C linker and missing-archive negative control", "dynamic-ELF negative control",
               "threads, panic recovery, persistence and queries", "empty chroot",
               "full-system QEMU Linux", "all-feature workspace nextest", "all-feature doctests",
               "store-disabled log compile", "release allocation gate"],
    "benchmarksRun": False,
    "scope": "static Rust core embedding and Linux executables; no new public C API or static Node executable",
}
(out / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
PY
(cd "$output" && sha256sum libbumbledb.a bumbledb-static-smoke bumbledb-log-duty vmlinuz-virt initramfs.cpio.gz notices.tar.gz static-library.json static-executables.json workspace-binaries.json qemu-verification.json verification.json > SHA256SUMS)
echo "static Linux ARM64 qualification passed; artifacts and evidence: $output"
