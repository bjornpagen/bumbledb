#!/usr/bin/env bash
set -euo pipefail

root="docs/free-join-paper-alignment-prds"
test -f "$root/README.md"

for raw in $(seq 0 22); do
  number=$(printf "%02d" "$raw")
  rg "\| $number \|.*\.md" "$root/README.md" >/dev/null
done

rg "docs/free-join-paper/audits|04-lftj-baseline-and-generic-join-special-case" "$root" && exit 1
exit 0
