#!/usr/bin/env bash
set -euo pipefail

suite_dir="docs/todos/rearchitecture_v2"
suite_readme="$suite_dir/README.md"
graph="$suite_dir/13_dependency_graph_and_migration_plan.md"

missing=0

for prd in "$suite_dir"/[0-9][0-9]_*.md; do
  name="$(basename "$prd")"
  if ! rg -q --fixed-strings "$name" "$suite_readme"; then
    printf 'missing from suite README: %s\n' "$name" >&2
    missing=1
  fi
  if ! rg -q --fixed-strings "$name" "$graph"; then
    printf 'missing from dependency graph/status: %s\n' "$name" >&2
    missing=1
  fi
done

if ! rg -q --fixed-strings "scripts/check-prd-map.sh" "$graph"; then
  printf 'dependency graph does not mention scripts/check-prd-map.sh\n' >&2
  missing=1
fi

if ! rg -q --fixed-strings "Global Stop Conditions" "$graph"; then
  printf 'dependency graph is missing global stop conditions\n' >&2
  missing=1
fi

exit "$missing"
