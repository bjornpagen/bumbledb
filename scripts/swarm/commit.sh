#!/usr/bin/env bash
# Commit only the given paths on main, leaving every other agent's working-tree
# changes untouched. Retries while another agent holds the git index lock.
#
#   scripts/swarm/commit.sh "<message>" <path>...
#
# Deleted after the cutover.
set -euo pipefail

repo=$(git rev-parse --show-toplevel)
msg=${1:?usage: commit.sh "<message>" <path>...}
shift
[ $# -gt 0 ] || { echo "commit.sh: no paths" >&2; exit 2; }
cd "$repo"

trailer=$'\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>'
for attempt in $(seq 1 60); do
	if git add -A -- "$@" 2>/dev/null && git commit -q -m "$msg$trailer" -- "$@"; then
		git log -1 --format='%h %s'
		exit 0
	fi
	if [ -e .git/index.lock ]; then
		sleep 1
		continue
	fi
	if git diff --quiet HEAD -- "$@" && [ -z "$(git ls-files -o --exclude-standard -- "$@")" ]; then
		echo "commit.sh: nothing to commit in the given paths" >&2
		exit 1
	fi
	echo "commit.sh: commit failed (attempt $attempt)" >&2
	sleep 1
done
echo "commit.sh: gave up after 60 attempts" >&2
exit 1
