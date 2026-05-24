#!/usr/bin/env bash
set -euo pipefail

rg "FreeJoinPlan|free_join_node|execute_free_join|bind_vars|lftj_runtime|lftj_access|sorted_trie|query_image|QueryImage|AccessLayout|IndexDescriptor|range_indexed|TimestampMicros|Decimal|NominalId|BenchmarkComparison|sqlite_count" crates && exit 1
exit 0
