#!/usr/bin/env bash
# The body of each CI job, so a local run of a lane is exactly what CI runs.
#
#   scripts/ci.sh <lane>
#
# Lanes are the lane_* functions below. Workflows only prepare toolchains,
# caches, services and credentials, then call one lane.
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

# Builds the addon with cargo profile $1 into $target_dir/addon/bumbledb.<platform>.node
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
	cp "$target_dir/$profile/$library" "$target_dir/addon/bumbledb.$platform.node"
	cp "$target_dir/addon/bumbledb.$platform.node" "ts/bumbledb.$platform.node"
}

lane_lint() {
	cargo fmt --all --check
	clippy
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
	cargo deny check --config .config/deny.toml
	cargo shear
	cargo bench --locked --workspace --no-run --profile ci
	python3 -m unittest discover -s scripts -p 'test_*.py'
	python3 scripts/static-linux-arm64/test_verify_elf.py
	python3 scripts/flame.py selftest
	python3 scripts/structural-corpus.py
	ts_install
	pnpm --dir ts run lint
	pnpm --dir ts run typecheck
}

lane_test() {
	cargo nextest run --locked --workspace --cargo-profile ci --profile ci
	cargo test --locked --workspace --doc --profile ci
	tree_clean
}

# BUMBLEDB_ADDON_PROFILE picks the cargo profile: addon-ci by default,
# release (fat LTO) for the artifacts main and releases ship.
lane_addon() {
	build_addon "${BUMBLEDB_ADDON_PROFILE:-addon-ci}"
	ts_install
	pnpm --dir ts test
	tree_clean
}

lane=${1:-}
if [ $# -ne 1 ] || ! declare -F "lane_$lane" >/dev/null; then
	echo "usage: scripts/ci.sh <$(compgen -A function lane_ | sed 's/^lane_//' | paste -sd '|' -)>" >&2
	exit 2
fi
"lane_$lane"
