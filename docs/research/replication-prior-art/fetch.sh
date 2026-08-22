#!/usr/bin/env bash
# Populate the replication-prior-art corpus. Run OUTSIDE the agent sandbox
# (it blocks arxiv.org et al.): `bash docs/research/replication-prior-art/fetch.sh`
# Idempotent; each paper lands in its own directory beside this script.
# Fault-tolerant: one rotted URL never blocks the rest; failures are
# reported at the end. Mirrors are tried in order.
set -uo pipefail
cd "$(dirname "$0")"

failed=()

get() { # get <dir> <url> [mirror...]
  local dir="$1"; shift
  mkdir -p "$dir"
  [ -s "$dir/paper.pdf" ] && return 0
  local url
  for url in "$@"; do
    echo "fetching $dir <- $url"
    if curl -fsSL --retry 2 --connect-timeout 20 -o "$dir/paper.pdf" "$url"; then
      [ -s "$dir/paper.pdf" ] && return 0
    fi
    rm -f "$dir/paper.pdf"
  done
  failed+=("$dir")
  return 0
}

# A — invariant-driven coordination avoidance
get arXiv-1402.2237-i-confluence \
  "https://arxiv.org/pdf/1402.2237"
get whittaker-interactive-checks \
  "http://www.vldb.org/pvldb/vol12/p14-whittaker.pdf" \
  "https://mwhittaker.github.io/publications/segmented_iconfluence.pdf"
get arXiv-1901.01930-calm \
  "https://arxiv.org/pdf/1901.01930"

# B — commutativity, escrow, reservations
# O'Neil 1986 (escrow) is ACM-paywalled: fetch via library; notes in THESIS.md.
get indigo-eurosys15 \
  "https://asc.di.fct.unl.pt/~nmp/pubs/eurosys-2015.pdf"
get redblue-osdi12 \
  "https://www.usenix.org/system/files/conference/osdi12/osdi12-final-162.pdf"
get homeostasis-sigmod15 \
  "https://www.cs.cornell.edu/~jnfoster/papers/homeostasis.pdf" \
  "https://infoscience.epfl.ch/record/207756/files/homeostasis-sigmod2015.pdf"

# C — deterministic replay
get calvin-sigmod12 \
  "http://cs.yale.edu/homes/thomson/publications/calvin-sigmod12.pdf" \
  "https://www.cs.umd.edu/~abadi/papers/calvin-sigmod12.pdf"
get aria-vldb20 \
  "http://www.vldb.org/pvldb/vol13/p2047-lu.pdf" \
  "https://www.vldb.org/pvldb/vol13/p2047-lu.pdf"

# D — the log on object storage
get aurora-sigmod17 \
  "https://web.stanford.edu/class/cs245/readings/aurora.pdf" \
  "https://pages.cs.wisc.edu/~yxy/cs764-f20/papers/aurora-sigmod-17.pdf" \
  "https://media.amazonwebservices.com/blog/2017/aurora-design-considerations-paper.pdf"

get delta-lake-vldb20 \
  "https://www.vldb.org/pvldb/vol13/p3411-armbrust.pdf" \
  "http://www.vldb.org/pvldb/vol13/p3411-armbrust.pdf"

# E — the dependency-theoretic frame
# The TCS full version (39pp) — strictly better than the ICDT'03 extended abstract.
get data-exchange-icdt03 \
  "https://www.cis.upenn.edu/~val/CIS650/DataX-tcs.pdf" \
  "https://link.springer.com/content/pdf/10.1007/3-540-36285-1_14.pdf"
get feral-sigmod15 \
  "http://www.bailis.org/papers/feral-sigmod2015.pdf" \
  "https://www.bailis.org/papers/feral-sigmod2015.pdf"

# F — the CRDT boundary
get arXiv-1806.10254-crdt-overview \
  "https://arxiv.org/pdf/1806.10254"
get arXiv-2210.12605-keep-calm-crdt \
  "https://arxiv.org/pdf/2210.12605"

echo
echo "done."
if [ "${#failed[@]}" -gt 0 ]; then
  echo "FAILED (all mirrors): ${failed[*]}"
  echo "canonical citations live in README.md — search the title; all except O'Neil have open PDFs somewhere stable."
else
  echo "all papers present:"
fi
ls -l */paper.pdf 2>/dev/null || true
