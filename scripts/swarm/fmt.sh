#!/usr/bin/env bash
# Format only the given Rust files (child modules are not followed), so a lane
# never reformats another lane's files on the shared tree.
#
#   scripts/swarm/fmt.sh <file.rs>...        (add --check to verify)
#
# Deleted after the cutover.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(git rev-parse --show-toplevel)"
exec rustfmt --edition 2024 --unstable-features --config skip_children=true "$@"
