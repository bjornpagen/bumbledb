#!/usr/bin/env bash
set -euo pipefail

pattern="FreeJoinPlan|free_join_node|execute_free_join|bind_vars|lftj_runtime|lftj_access|sorted_trie|query_image|QueryImage|AccessLayout|IndexDescriptor|range_indexed|TimestampMicros|Decimal|NominalId|BenchmarkCompariso[n]|sqlite_count"
rg "$pattern" crates && exit 1
exit 0
