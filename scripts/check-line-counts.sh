#!/usr/bin/env bash
set -euo pipefail

status=0

while IFS= read -r file; do
  lines=$(wc -l <"$file")
  limit=500
  case "$file" in
    *tests/*|*_tests.rs|*query_test_helpers.rs)
      limit=700
      ;;
  esac
  if (( lines > limit )); then
    printf '%s has %d lines, over limit %d\n' "$file" "$lines" "$limit" >&2
    status=1
  fi
done < <(find crates -name '*.rs' -type f | sort)

exit "$status"
