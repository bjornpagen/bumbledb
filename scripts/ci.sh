#!/usr/bin/env bash
# The body of each CI job, so a local run of a lane is exactly what CI runs.
#
#   scripts/ci.sh <lane>
#
# Lanes are the lane_* functions below. Workflows only prepare toolchains
# and caches, then call one lane.
set -euo pipefail
cd "$(dirname "$0")/.."
export PYTHONDONTWRITEBYTECODE=1

target_dir=${CARGO_TARGET_DIR:-$PWD/target}

host_platform() {
	node -p 'process.platform + "-" + process.arch'
}

clippy() {
	cargo clippy --locked --workspace --all-targets -- -D warnings
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
}

ts_install() {
	pnpm --dir ts install --frozen-lockfile
}

# Fails when a lane modified a tracked file or left an unignored one behind.
tree_clean() {
	git diff --exit-code
	local untracked
	untracked=$(git ls-files --others --exclude-standard)
	if [ -n "$untracked" ]; then
		printf 'untracked files after the lane:\n%s\n' "$untracked" >&2
		return 1
	fi
}

# Builds the addon with cargo profile $1 into $target_dir/addon/bdb.<platform>.node
# and installs it as the dev addon that ts/src/native/load.ts prefers.
build_addon() {
	local profile=$1 platform library
	platform=$(host_platform)
	cargo build --locked -p bumbledb-node --profile "$profile"
	case "$platform" in
	darwin-*) library=libbumbledb_node.dylib ;;
	linux-*) library=libbumbledb_node.so ;;
	*) echo "ci.sh: no addon for $platform" >&2; return 1 ;;
	esac
	mkdir -p "$target_dir/addon"
	cp "$target_dir/$profile/$library" "$target_dir/addon/bdb.$platform.node"
	cp "$target_dir/addon/bdb.$platform.node" "ts/bdb.$platform.node"
}

lane_lint() {
	cargo fmt --all --check
	clippy
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
	cargo deny check
	cargo shear
	cargo bench --locked --workspace --no-run --profile ci
	python3 -m unittest discover -s scripts -p 'test_*.py'
	python3 scripts/static-linux-arm64/test_verify_elf.py
	python3 scripts/flame.py selftest
	python3 scripts/structural-corpus.py
	ts_install
	pnpm --dir ts run lint
	pnpm --dir ts run typecheck
	crates/bumbledb-node/dts.sh --check
}

lane_test() {
	cargo nextest run --locked --workspace --cargo-profile ci --profile ci
	cargo test --locked --workspace --doc --profile ci
	tree_clean
}

# Builds the addon with cargo profile $1, then packs and smoke-tests the host's
# package family from its tarballs.
addon() {
	build_addon "$1"
	ts_install
	pnpm --dir ts test
	rm -rf "$target_dir/family"
	node scripts/family.mjs pack "$target_dir/addon" "$target_dir/family"
	node scripts/family.mjs smoke "$target_dir/family"
	tree_clean
}

lane_addon() {
	addon addon-ci
}

# Fat LTO: the artifacts main and releases ship.
lane_addon_release() {
	addon release
}

# Miri runs the engine's unit tests; tests that reach LMDB or are too slow
# under the interpreter carry #[cfg_attr(miri, ignore)]. CI runs one lane per
# shard, miri_1 through miri_4.
miri() {
	cargo miri nextest run --locked -p bumbledb --lib --partition "hash:$1/4"
}

lane_miri_1() { miri 1; }
lane_miri_2() { miri 2; }
lane_miri_3() { miri 3; }
lane_miri_4() { miri 4; }

# The deep profile adds the wide sweeps in `deep` test modules; the gate cargo
# profile gives release semantics, which also runs the release-only allocation gates.
lane_deep() {
	cargo nextest run --locked --workspace --cargo-profile gate --profile deep
}

lane_clippy() {
	clippy
}

lane_udeps() {
	cargo udeps --locked --workspace --all-targets --all-features
}

# The hand-tuned NEON kernels must stay free of scalar flag writers in the shipped build.
lane_asm() {
	cargo build --locked -p bumbledb-bench --release
	scripts/check-asm.sh "$target_dir/release/bumbledb-bench"
}

lane=${1:-}
if [ $# -ne 1 ] || ! declare -F "lane_$lane" >/dev/null; then
	echo "usage: scripts/ci.sh <$(compgen -A function lane_ | sed 's/^lane_//' | paste -sd '|' -)>" >&2
	exit 2
fi
"lane_$lane"
