#!/usr/bin/env python3
"""Read every P3 round/control and preserve descriptive timing comparisons."""
import argparse
import math
from diagnostics import ROUND, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
path = ROUND/f'p3-comparison-{args.attempt}'
state = load(path/'STATE.json')
assert state['status'] == 'MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
reports = {}
flags = []
retries = []
order = state['order']
arms = list(dict.fromkeys(label[0] for label in order))
assert arms in (['A', 'B'], ['A', 'B', 'C']), arms
assert sorted(order) == sorted(arm + str(round_) for arm in arms for round_ in (0, 1))
boundaries = {'ghz_ours': 0, 'ghz_theirs': 0}
names = state['expected_scenarios'] + state['expected_reads']
for label in order:
    scenarios = load(path/label/'scenarios/scenarios.json')
    reads = load(path/label/'reads/report.json')
    reports[label] = {row['name']: row for row in scenarios['queries'] + reads['reads']}
    assert list(reports[label]) == names
    for row in reads['reads']:
        for kind in ['ghz_ours', 'ghz_theirs']:
            boundaries[kind] += 1
            if row[kind]['contaminated']:
                flags.append((label, row['name'], kind, row[kind]))
            if row[kind]['retried']:
                retries.append((label, row['name'], kind, row[kind]))
for numerator, denominator in [('B', 'A')] + ([('C', 'A'), ('C', 'B')] if 'C' in arms else []):
    print(f'\n{numerator}/{denominator}')
    print('| Family | p50 | mean | p90 | p95 | p99/max |')
    print('|---|---:|---:|---:|---:|---:|')
    for name in names:
        ratios = []
        for stat in ['p50', 'mean_ns', 'p90', 'p95', 'max']:
            center = {key: math.sqrt(reports[key+'0'][name]['ours'][stat] *
                                     reports[key+'1'][name]['ours'][stat]) for key in arms}
            ratios.append(center[numerator]/center[denominator])
        print('|', name, '|', ' | '.join(f'{ratio:.4f}' for ratio in ratios), '|')
print('\nAll ordinary round summaries in execution order (ns, nothing dropped):')
for name in names:
    for label in order:
        row = reports[label][name]
        assert row['ours']['p99'] == row['ours']['max']
        print('ROUND', label, name, row['ours'], row.get('ghz_ours'))
print('Clock boundaries:', boundaries)
print('Clock flags (ours and SQLite retained separately):', flags)
print('Harness retries (including subsequently unflagged boundaries):', retries)
print('Descriptive round centers, no sample independence/significance or frequency normalization.')
