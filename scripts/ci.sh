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

require_env() {
	local name missing=
	for name; do
		[ -n "${!name:-}" ] || missing="$missing $name"
	done
	if [ -n "$missing" ]; then
		echo "ci.sh: missing configuration:$missing" >&2
		return 1
	fi
}

s3api() {
	aws s3api --region "$BUMBLEDB_S3_REGION" ${BUMBLEDB_S3_ENDPOINT:+--endpoint-url "$BUMBLEDB_S3_ENDPOINT"} "$@"
}

# Contract probe: 32 racing creates of one key. Exactly one wins, every loser
# is refused as a conditional-write conflict, and the stored bytes are the winner's.
s3_race() {
	local bucket=$1 key="${BUMBLEDB_S3_PREFIX}race" dir i winner= pids=()
	dir=$(mktemp -d)
	for i in $(seq 1 32); do
		printf 'contender %s\n' "$i" > "$dir/$i.body"
		s3api put-object --bucket "$bucket" --key "$key" --body "$dir/$i.body" --if-none-match '*' \
			> /dev/null 2> "$dir/$i.err" &
		pids+=($!)
	done
	for i in $(seq 1 32); do
		if wait "${pids[$((i - 1))]}"; then
			if [ -n "$winner" ]; then
				echo "s3 race on $bucket: contenders $winner and $i both created $key" >&2
				return 1
			fi
			winner=$i
		elif ! grep -Eq 'PreconditionFailed|ConditionalRequestConflict' "$dir/$i.err"; then
			echo "s3 race on $bucket: contender $i failed without a conditional-write refusal:" >&2
			cat "$dir/$i.err" >&2
			return 1
		fi
	done
	if [ -z "$winner" ]; then
		echo "s3 race on $bucket: no contender created $key" >&2
		return 1
	fi
	s3api get-object --bucket "$bucket" --key "$key" "$dir/stored" > /dev/null
	if ! cmp -s "$dir/stored" "$dir/$winner.body"; then
		echo "s3 race on $bucket: the stored object is not the winner's" >&2
		return 1
	fi
	rm -rf "$dir"
	echo "s3 race on $bucket: 1 of 32 creates won"
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
	crates/bumbledb-node/dts.sh --check
}

lane_test() {
	cargo nextest run --locked --workspace --cargo-profile ci --profile ci
	cargo test --locked --workspace --doc --profile ci
	tree_clean
}

# BUMBLEDB_ADDON_PROFILE picks the cargo profile: addon-ci by default,
# release (fat LTO) for the artifacts main and releases ship. The host's
# package family is packed and smoke-tested from its tarballs.
lane_addon() {
	build_addon "${BUMBLEDB_ADDON_PROFILE:-addon-ci}"
	ts_install
	pnpm --dir ts test
	rm -rf "$target_dir/family"
	node scripts/family.mjs pack "$target_dir/addon" "$target_dir/family"
	node scripts/family.mjs smoke "$target_dir/family"
	tree_clean
}

# Real-store conformance. BUMBLEDB_S3_TARGET is seaweedfs (BUMBLEDB_S3_ENDPOINT
# required) or aws (the log bucket is an S3 Express directory bucket). Both need
# BUMBLEDB_S3_REGION, BUMBLEDB_S3_LOG_BUCKET, BUMBLEDB_S3_CKPT_BUCKET, a fresh
# BUMBLEDB_S3_PREFIX ending in "/", and AWS credentials in the environment.
lane_s3() {
	require_env BUMBLEDB_S3_TARGET BUMBLEDB_S3_REGION BUMBLEDB_S3_LOG_BUCKET BUMBLEDB_S3_CKPT_BUCKET BUMBLEDB_S3_PREFIX
	case "$BUMBLEDB_S3_TARGET:${BUMBLEDB_S3_ENDPOINT:+endpoint}" in
	seaweedfs:endpoint) ;;
	aws:)
		case "$BUMBLEDB_S3_LOG_BUCKET" in
		*--x-s3) ;;
		*) echo "ci.sh: BUMBLEDB_S3_LOG_BUCKET must be an S3 Express directory bucket (*--x-s3)" >&2; return 1 ;;
		esac
		;;
	*) echo "ci.sh: BUMBLEDB_S3_TARGET must be seaweedfs (with BUMBLEDB_S3_ENDPOINT) or aws (without)" >&2; return 1 ;;
	esac
	case "$BUMBLEDB_S3_PREFIX" in
	*/) ;;
	*) echo "ci.sh: BUMBLEDB_S3_PREFIX must end in /" >&2; return 1 ;;
	esac
	s3_race "$BUMBLEDB_S3_LOG_BUCKET"
	s3_race "$BUMBLEDB_S3_CKPT_BUCKET"
	build_addon addon-ci
	ts_install
	pnpm --dir ts run test:s3
}

# Miri runs the engine's unit tests; tests that reach LMDB or are too slow
# under the interpreter carry #[cfg_attr(miri, ignore)].
lane_miri() {
	cargo miri nextest run --locked -p bumbledb --lib
}

# BUMBLEDB_DEEP=1 widens the property and differential sweeps; the gate profile
# gives release semantics, which also runs the release-only allocation gates.
lane_deep() {
	BUMBLEDB_DEEP=1 cargo nextest run --locked --workspace --cargo-profile gate --profile deep
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
