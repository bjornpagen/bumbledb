#!/usr/bin/env python3
"""Recompute all raw distributions and report every matched control/clock flag."""
import argparse
import json
import math
from diagnostics import ROUND, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
path = ROUND/f'g2-timing-{args.attempt}'
state = load(path/'STATE.json')
assert state['status'] == 'PER-DRAW-MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
assert all(step['status'] == 'PASS' for step in state['steps'])
assert state['processes_completed'] == 32 and state['distributions'] == 128
cases = [(family, draw) for draw in range(4) for family in ['triangle', 'point']]
order = ['A0', 'B0', 'B1', 'A1']
rows = {}
metadata = {}
stamps = {}
for family, draw in cases:
    for label in order:
        name = f'{family}-{draw}-{label}'
        content = (path/f'{name}.log').read_text()
        assert 'scheduler boost: user-interactive QoS verified (not hard P-core affinity)' in content
        records = [json.loads(line) for line in content.splitlines() if line.startswith('{')]
        assert len(records) == 5
        meta, *blocks = records
        if (family, draw) not in metadata:
            metadata[family, draw] = meta
        assert meta == metadata[family, draw]
        assert blocks[0]['ghz'] == blocks[1]['ghz']
        for block in blocks:
            raw, batch = block['raw_ns'], block['batch']
            assert all(isinstance(value, int) and value > 0 for value in raw)
            assert len(raw) == (16 if block['kind'] in ['cold', 'second'] else 64)
            assert batch == (16 if block['kind'] == 'memoized' else 1)
            values = sorted(ns // batch for ns in raw)
            computed = dict(min=min(values), max=max(values), mean_ns=sum(values)//len(values))
            for percentile in [50, 90, 95, 99]:
                computed[f'p{percentile}'] = values[math.ceil(percentile * len(values) / 100) - 1]
            assert block['stats'] == computed, (name, block['kind'])
            ghz = block['ghz']
            assert all(math.isfinite(ghz[field]) and ghz[field] > 0 for field in ['pre', 'post', 'threshold'])
            assert ghz['contaminated'] == (min(ghz['pre'], ghz['post']) < ghz['threshold'])
            assert not ghz['retried']
            rows[family, draw, label, block['kind']] = block
            if block['kind'] != 'second':
                stamps[name, block['kind']] = ghz

flagged = [(key, value) for key, value in stamps.items() if value['contaminated']]
assert len(rows) == 128 and len(stamps) == 96
print('CLOCK-BRACKETS', len(stamps), 'flagged', len(flagged), 'retried', 0)
for key, stamp in stamps.items():
    print('CLOCK', key, stamp)

for family, draw in cases:
    print('CASE', family, draw, metadata[family, draw])
    for kind in ['cold', 'second', 'rotating', 'memoized']:
        group = {label: rows[family, draw, label, kind] for label in order}
        ratios = {}
        for metric in ['p50', 'mean_ns', 'p90', 'p95', 'p99', 'max']:
            centers = {key: math.sqrt(group[key+'0']['stats'][metric] * group[key+'1']['stats'][metric])
                       for key in ['A', 'B']}
            ratios[metric] = centers['B'] / centers['A']
        p50s = {label: group[label]['stats']['p50'] for label in order}
        means = {label: group[label]['stats']['mean_ns'] for label in order}
        bracket_flags = {label: group[label]['ghz']['contaminated'] for label in order}
        print('DISTRIBUTION', family, draw, kind, 'p50s', p50s, 'means', means,
              'B/A', ratios, 'flags', bracket_flags)
        for label, block in group.items():
            raw = block['raw_ns']
            print('RAW-CHECK', family, draw, kind, label, 'n', len(raw), 'batch', block['batch'],
                  'distinct', len(set(raw)), 'elapsed_sum_ns', sum(raw), 'stats', block['stats'])
print('PASS: all 128 distributions recomputed from raw samples; 96 clock brackets reviewed; no samples dropped.')
print('Descriptive ABBA centers only. Separate empty/nonempty and tiny timer-quantized controls; no Pi or full-suite claim.')
