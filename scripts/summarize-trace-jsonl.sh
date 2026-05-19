#!/usr/bin/env bash
set -euo pipefail

trace_path="${1:?trace jsonl path required}"
result_path="${2:-}"

printf '== Trace file ==\n'
wc -l "$trace_path"
du -h "$trace_path"

if [[ -n "$result_path" && -f "$result_path" ]]; then
  printf '\n== Query summary ==\n'
  jq -r '.results[] | [.dataset,.query,.runtime,.plan_family,.bumbledb.avg_us,.sqlite.avg_us,.query_image_built_during_query,.counters.direct_kernel_rows,.counters.hash_index_build_rows,.counters.sorted_trie_builds,.counters.materialized_output_values,.gate.passed] | @tsv' "$result_path"
fi

printf '\n== Span busy time by name ==\n'
jq -r '
  def micros:
    if test("ms$") then (sub("ms$"; "") | tonumber * 1000)
    elif test("µs$") then (sub("µs$"; "") | tonumber)
    elif test("ns$") then (sub("ns$"; "") | tonumber / 1000)
    elif test("s$") then (sub("s$"; "") | tonumber * 1000000)
    else 0 end;
  select(.fields.message == "close" and .span.name != null)
  | [.span.name, (.fields["time.busy"] | micros)]
  | @tsv
' "$trace_path" \
  | awk -F '\t' '{sum[$1]+=$2; count[$1]+=1} END {for (s in sum) printf "%s\t%d\t%.0f\t%.2f\n", s, count[s], sum[s], sum[s]/count[s]}' \
  | sort -k3,3nr \
  | head -40
