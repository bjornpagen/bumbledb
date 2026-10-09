#!/usr/bin/env bash
# Run a command against committed HEAD plus this agent's owned paths only, so
# other agents' uncommitted edits on the shared main tree cannot break it.
#
#   scripts/swarm/sandbox.sh <label> <owned-path>... -- <command...>
#
# The sandbox lives at target/swarm/<label>/src and is updated in place with
# rsync --checksum, so unchanged files keep their mtimes and cargo stays
# incremental. Each label gets its own CARGO_TARGET_DIR.
# Deleted after the cutover.
set -euo pipefail

repo=$(git rev-parse --show-toplevel)
label=${1:?usage: sandbox.sh <label> <owned-path>... -- <command...>}
shift
owned=()
while [ $# -gt 0 ] && [ "$1" != "--" ]; do
	owned+=("${1%/}")
	shift
done
[ "${1:-}" = "--" ] || { echo "sandbox.sh: missing -- before the command" >&2; exit 2; }
shift
[ $# -gt 0 ] || { echo "sandbox.sh: missing command" >&2; exit 2; }

root="$repo/target/swarm/$label"
stage="$root/stage"
snap="$root/src"
excludes=(--exclude node_modules --exclude target --exclude dist --exclude .git)

rm -rf "$stage"
mkdir -p "$stage" "$snap"
git -C "$repo" archive HEAD | tar -x -C "$stage"
for p in "${owned[@]+"${owned[@]}"}"; do
	rm -rf "${stage:?}/$p"
	if [ -d "$repo/$p" ]; then
		mkdir -p "$stage/$p"
		rsync -a "${excludes[@]}" "$repo/$p/" "$stage/$p/"
	elif [ -e "$repo/$p" ]; then
		mkdir -p "$(dirname "$stage/$p")"
		cp -p "$repo/$p" "$stage/$p"
	fi
done
rsync -a --checksum --delete "${excludes[@]}" "$stage/" "$snap/"
rm -rf "$stage"

cd "$snap"
export CARGO_TARGET_DIR="$root/target"
exec "$@"
