#!/usr/bin/env python3
"""Reconcile Q2 allocation requests to every observed map, with schema checks."""
import argparse
import ast
import re
import shutil
from diagnostics import Phase, ROUND, digest, load
from q2_experiment import fingerprint, prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('q1', type=int)
parser.add_argument('q2', type=int)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
paths = {v: ROUND/f'q2-controls-{v}-{getattr(args, v)}' for v in ['q1', 'q2']}
states = {v: load(p/'STATE.json') for v, p in paths.items()}
assert all(s['status'] == 'Q2-ALLOCATION-CONTROLS-COMPLETE-REVIEW-REQUIRED' for s in states.values())
assert states['q1']['source_tree'] == states['q2']['source_tree']
assert states['q1']['binary_sha256'] != states['q2']['binary_sha256']
assert states['q1']['input_hashes'] == states['q2']['input_hashes']
assert fingerprint() == states['q2']['ordinary_source']
common = set(states['q1']['dependencies']) & set(states['q2']['dependencies'])
assert len(common) == 5
assert all(states['q1']['dependencies'][n] == states['q2']['dependencies'][n] == digest(n) for n in common)
assert all(not s['engine_artifact']['fresh'] for s in states.values())
phase = Phase(f'q2-controls-review-{args.attempt}')
shutil.copy2(__file__, phase.path/'review.py')

def parse(raw):
    alloc_rows = re.findall(r'ALLOC (\S+) AllocWindow \{ ([^}]+) \}', raw)
    alloc = {label: tuple(map(int, re.findall(r': (\d+)', data))) for label, data in alloc_rows}
    assert len(alloc) == len(alloc_rows) == 328 and all(len(a) == 4 for a in alloc.values())
    map_rows = re.findall(r'^MAP (\S+) role=(\S+) rows=(\d+) arity=(\d+) owners=(\[.*?\]) bytes=(\d+)$', raw, re.M)
    maps = {}
    for label, role, count, arity, owners, size in map_rows:
        owners, size = ast.literal_eval(owners), int(size)
        assert len(owners) == 5 and all(l <= cap for l, cap, _ in owners)
        assert sum(cap*width for _, cap, width in owners) == size
        maps[label, role] = (int(count), int(arity), owners, size)
    assert len(maps) == len(map_rows) == 488
    return alloc, maps

try:
    raw = {v: (p/'controls.log').read_text() for v, p in paths.items()}
    parsed = {v: parse(text) for v, text in raw.items()}
    alloc = {v: pair[0] for v, pair in parsed.items()}
    table = {v: pair[1] for v, pair in parsed.items()}
    assert alloc['q1'].keys() == alloc['q2'].keys()
    assert table['q1'].keys() == table['q2'].keys()
    # Fresh same-path Q1 control must reproduce the previously frozen Q1
    # ownership/request observations exactly, not just resemble them.
    saved = parse((ROUND/'q1-controls-candidate-2/controls.log').read_text())
    assert saved == parsed['q1']
    unchanged = {}
    for prefix in ['ANSWER', 'PROJECTION', 'SET', 'AGGREGATE', 'DENSE', 'FOLDS', 'AGGOWNERS', 'PASS']:
        lines = {v: re.findall(r'^'+prefix+r' .*$', text, re.M) for v, text in raw.items()}
        assert lines['q1'] == lines['q2'], prefix
        unchanged[prefix] = len(lines['q1'])
    retained = {v: {} for v in table}
    for variant, maps in table.items():
        for (label, role), (count, arity, owners, size) in maps.items():
            window = label.split('/')[0]
            retained[variant][window] = retained[variant].get(window, 0) + size
            other = table['q1'][label, role]
            assert (count, arity) == other[:2]
            assert [w for _, _, w in owners] == [w for _, _, w in other[2]]
            if ':prepare/' in label or ':release/' in label:
                assert size == 0
            if variant == 'q2':
                ctrl, keys, values, stamps, slots = owners
                assert keys[0] == count*arity and values[0] == count
                assert slots[0] == stamps[0]
                assert ctrl[0] == (slots[0]+7 if slots[0] else 0)
                assert count*3 <= slots[0]
                assert keys[2] == 8 and slots[2] == 4
    cases = {label.rsplit(':', 1)[0] for label in alloc['q1']}
    assert len(cases) == 41
    windows, summaries, warm_changes = {}, {}, {}
    for case in sorted(cases):
        cumulative = [0]*4
        for kind in ['prepare', 'cold', 'warm', 'small', 'return', 'release', 'refill', 'warm_refill']:
            label = f'{case}:{kind}'
            a, b = alloc['q1'][label], alloc['q2'][label]
            delta = tuple(y-x for x, y in zip(a, b))
            cumulative = [x+y for x, y in zip(cumulative, delta)]
            ra, rb = (retained[v].get(label, 0) for v in ['q1', 'q2'])
            assert cumulative[2]-cumulative[3] == rb-ra, (label, cumulative, ra, rb)
            if kind in ['warm', 'small', 'return', 'warm_refill'] and delta != (0, 0, 0, 0):
                warm_changes[label] = delta
            windows[label] = dict(q1=a, q2=b, delta=delta, retained_q1=ra, retained_q2=rb)
        p, c = windows[case+':prepare'], windows[case+':cold']
        summaries[case] = dict(prepare_delta=p['delta'], cold_delta=c['delta'],
            combined_delta=tuple(x+y for x, y in zip(p['delta'], c['delta'])),
            retained_q1=c['retained_q1'], retained_q2=c['retained_q2'])
        print(case, summaries[case])
    phase.state.update(sources={v: s['ordinary_source'] for v, s in states.items()},
        prior_identities=prior_identities(), paths={v: str(p) for v, p in paths.items()},
        log_hashes={v: digest(p/'controls.log') for v, p in paths.items()},
        schema={v: s['owner_schema'] for v, s in states.items()},
        cases=41, windows=windows, summaries=summaries, unchanged_records=unchanged,
        map_records=488, warm_changes=warm_changes, q1_saved_observations_reproduced=True,
        limitations='Actual requested layouts and retained backing; not RSS/peak/Pi or ordinary speed. No allocation-site/CPU/full trace. Different fifth-owner semantics explicit; all net bytes reconcile with map ownership, no unexplained residual.')
    assert fingerprint() == states['q2']['ordinary_source']
    print('WARM CHANGES', warm_changes)
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q2-ALL-OWNERS-REQUESTS-AND-REPRESENTATION-RECONCILED')
