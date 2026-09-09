#!/usr/bin/env python3
"""Compare actual cache payloads, not whole-query counts across P2/P3."""
import argparse
import ast
import json
import re
from diagnostics import ROUND, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseline', type=int)
parser.add_argument('candidate', type=int)
parser.add_argument('--baseline-compact', action='store_true')
args = parser.parse_args()
snapshots = {}
for role, attempt in [('baseline', args.baseline), ('candidate', args.candidate)]:
    path = ROUND/f'p3-owner-{attempt}'
    state = load(path/'STATE.json')
    assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
    log = (path/'ownership.log').read_text()
    record = {'binary_sha256': state['binary_sha256'], 'passes': {}}
    for label in ['cold', 'warm', 'released']:
        owners = {}
        for name, length, capacity, word, live, retained in re.findall(
                rf'^OWNER {label} (\w+) len=(\d+) capacity=(\d+) word_bytes=(\d+) live_bytes=(\d+) retained_bytes=(\d+)$', log, re.M):
            owners[name] = dict(zip(['len', 'capacity', 'word_bytes', 'live_bytes', 'retained_bytes'],
                                   map(int, [length, capacity, word, live, retained])))
        assert len(owners) == 9
        groups = []
        slabs = []
        for key, length, base, padded in re.findall(
                rf'^GROUP {label} key=(\[.*?\]) len=(\d+) tree_base=(\d+) p=(\d+)$', log, re.M):
            length, base, padded = map(int, [length, base, padded])
            compact = role == 'candidate' or args.baseline_compact
            words = length if compact and length <= 128 else 2*padded
            slabs.append((base, words))
            groups.append((ast.literal_eval(key), length, padded))
        assert len(groups) == 2000
        # Directories are allocated at first touch; slabs at second touch.
        # Those orders need not agree. Validate the observed physical order.
        expected_offset = 0
        for base, words in sorted(slabs):
            assert base == expected_offset
            expected_offset += words
        assert expected_offset == owners['tree']['len']
        costs = re.search(rf'^ALLOC {label} .*cache_probe=ProbeCosts \{{ ([^}}]+) \}}$', log, re.M)
        assert costs
        costs = {key: int(value) for key, value in re.findall(r'(\w+): (\d+)', costs[1])}
        assert costs['calls'] == 150034
        record['passes'][label] = dict(owners=owners, groups=groups, cache_probe=costs,
            live_bytes=sum(owner['live_bytes'] for owner in owners.values()),
            retained_bytes=sum(owner['retained_bytes'] for owner in owners.values()))
        if label != 'cold':
            for field in ['owners', 'groups']:
                assert record['passes'][label][field] == record['passes']['cold'][field]
    assert record['passes']['cold']['cache_probe'] == record['passes']['released']['cache_probe']
    snapshots[role] = record
for label in ['cold', 'warm', 'released']:
    before = snapshots['baseline']['passes'][label]
    after = snapshots['candidate']['passes'][label]
    assert before['groups'] == after['groups']
    assert {k: v for k, v in before['owners'].items() if k != 'tree'} == {
        k: v for k, v in after['owners'].items() if k != 'tree'}
    if args.baseline_compact:
        assert before['owners'] == after['owners']
        assert before['cache_probe'] == after['cache_probe']
    for row in [before, after]:
        row.pop('groups')
result = dict(snapshots=snapshots,
    caveat=('Both audits are standalone P3, with identical cache ownership required. '
            if args.baseline_compact else
            'Baseline audit 3 has P2 aggregate changes outside the unchanged cache; candidate is standalone P3. ')
           + 'Only identical observed cache owners/group ordering and scoped cache.probe '
           'requests are compared. Not whole-query allocations, RSS, mmap, allocator usable sizes or timing.')
before = snapshots['baseline']['passes']['cold']
after = snapshots['candidate']['passes']['cold']
result['reductions'] = {field: dict(bytes=before[field]-after[field],
                                  percent=100*(1-after[field]/before[field]))
                        for field in ['live_bytes', 'retained_bytes']}
print(json.dumps(result, indent=2))
