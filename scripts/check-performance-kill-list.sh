#!/usr/bin/env bash
set -euo pipefail

suite_dir="docs/todos/performance_kill_list"
roadmap="docs/todos/README.md"
suite_readme="$suite_dir/README.md"

missing=0

for prd in "$suite_dir"/[0-9][0-9]_*.md; do
  name="$(basename "$prd")"
  if ! rg -q --fixed-strings "$name" "$suite_readme"; then
    printf 'missing from performance kill-list README: %s\n' "$name" >&2
    missing=1
  fi
  if ! rg -q --fixed-strings "performance_kill_list/$name" "$roadmap"; then
    printf 'missing from roadmap: performance_kill_list/%s\n' "$name" >&2
    missing=1
  fi
done

if ! rg -q --fixed-strings "Trace source" "$suite_readme"; then
  printf 'performance kill-list README is missing trace source\n' >&2
  missing=1
fi

exit "$missing"
