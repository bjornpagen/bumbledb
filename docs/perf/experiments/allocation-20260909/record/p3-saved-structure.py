#!/usr/bin/env python3
"""Count saved temporal corpus groups; structural estimates, no trace/timing."""
import argparse
import json
import shutil
from diagnostics import Phase, REPO, ROUND, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'p3-saved-structure-{args.attempt}')
shutil.copy2(__file__, phase.path / 'structure.py')
corpus = ROUND / 'data/scenarios/scenarios/temporal/oracle.sqlite'
manifest = load(ROUND / 'full/MANIFEST.json')
assert manifest['status'] == 'LOCAL-LANES-COMPLETE'
assert str(ROUND / 'data/scenarios') in manifest['lanes']['scenarios']['command']
fingerprint = digest(corpus)
phase.state.update(
    corpus=str(corpus), corpus_sha256=fingerprint,
    ordinary_manifest_sha256=digest(ROUND / 'full/MANIFEST.json'),
    overlap_source_sha256=digest(REPO / 'crates/bumbledb/src/interval/overlap.rs'),
    limitation='Actual SQL per-key group sizes; projected tree/end live words '
               'IF one full cache group per key is built. Not measured cache '
               'directories, retained capacity, Rust allocations, RSS or timing.',
)
phase.save()
try:
    phase.run('schema', ['sqlite3', '-readonly', corpus, '.schema'])
    phase.run('groups', ['sqlite3', '-readonly', '-json', corpus,
                        'SELECT "key", count(*) AS n FROM "Span" '
                        'GROUP BY "key" ORDER BY "key"'])
    groups = json.loads((phase.path / 'groups.log').read_text())
    counts = [row['n'] for row in groups]
    assert [row['key'] for row in groups] == list(range(2000))
    assert sum(counts) == 150034
    small = [n for n in counts if n <= 128]
    large = [n for n in counts if n > 128]
    tree_words = lambda values: sum(2 * (1 << (n - 1).bit_length()) for n in values)
    old = tree_words(counts)
    proposed = sum(small) + tree_words(large)
    result = dict(groups=len(counts), positions=sum(counts),
                  min_group=min(counts), max_group=max(counts),
                  flat_groups=len(small), flat_positions=sum(small),
                  flat_min=min(small), flat_max=max(small), large_groups=large,
                  current_tree_words=old, flat_only_end_words_projection=proposed,
                  unused_flat_tree_words=old - proposed,
                  projected_live_payload_bytes_removed=8 * (old - proposed))
    phase.state['result'] = result
    phase.save()
    print(json.dumps(result, indent=2), flush=True)
    assert digest(corpus) == fingerprint
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('SAVED-CORPUS-STRUCTURE-REVIEW-REQUIRED')
