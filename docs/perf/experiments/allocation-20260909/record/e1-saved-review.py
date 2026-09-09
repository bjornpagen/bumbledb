#!/usr/bin/env python3
"""Compare every saved execution and COLT owner, not a pooled memory total."""
import argparse
import ast
import re
from diagnostics import ROUND, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
paths = {'baseline': ROUND/'m1-saved-baseline-1', 'candidate': ROUND/f'e1-saved-{args.attempt}'}
states = {name: load(path/'STATE.json') for name, path in paths.items()}
for state in states.values():
    assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
    assert all(step['status'] == 'PASS' for step in state['steps'])
for field in ['dependencies', 'database_sha256', 'oracle_sha256']:
    assert states['baseline'][field] == states['candidate'][field]
logs = {name: (path/'ownership.log').read_text() for name, path in paths.items()}
labels = [f'{kind}-{key}' for key in [1, 167, 333, 500] for kind in ['cold', 'warm', 'repeat']]
labels += [f'rotation{cycle}-{key}' for cycle in range(2) for key in [1, 167, 333, 500]]
parsed = {}
for name, content in logs.items():
    executions = re.findall(r'EXEC ([^ ]+) lo=(\d+) hi=(\d+) answers=(\d+) '
        r'alloc=AllocWindow \{ allocs: (\d+), deallocs: (\d+), alloc_bytes: (\d+), dealloc_bytes: (\d+) \} '
        r'clones=\((\d+), (\d+), (\d+)\)', content)
    assert [row[0] for row in executions] == labels
    execs = {row[0]: tuple(map(int, row[1:])) for row in executions}
    for label, values in execs.items():
        assert values[2] == (0 if values[0] == 500 else 5)
        assert values[7:] == (0, 0, 0)
        if label.startswith(('warm-', 'repeat-', 'rotation1-')):
            assert values[3:7] == (0, 0, 0, 0)
    owners = {}
    for row in re.findall(r'^COLT ([^ ]+) occ=(\d+) slot=(\d+) rows=(\d+) maps=(\d+) '
        r'owners=(\[.*?\]) table_bytes=(\d+) arena_bytes=(\d+) capacity_bytes=(\d+) '
        r'retained=(\d+) histogram=(\{.*?\})$', content, re.MULTILINE):
        key = (row[0], int(row[1]), int(row[2]))
        assert key not in owners
        owners[key] = (int(row[3]), int(row[4]), ast.literal_eval(row[5]),
                       *map(int, row[6:10]), ast.literal_eval(row[10]))
    assert len(owners) == 78
    totals = {}
    for label, values in re.findall(r'^TOTAL ([^ ]+) (.+)$', content, re.MULTILINE):
        assert label not in totals
        totals[label] = {field: int(value) for field, value in re.findall(r'(\w+)=(\d+)', values)}
    assert list(totals) == labels
    for label in labels:
        snaps = [snap for (window, _, _), snap in owners.items() if window == label]
        total = totals[label]
        assert total['colts'] == len(snaps)
        assert total['table_bytes'] == sum(s[3] for s in snaps)
        assert total['arena_bytes'] == sum(s[4] for s in snaps)
        assert total['capacity_bytes'] == sum(s[5] for s in snaps)
        assert total['retained'] == sum(s[6] for s in snaps)
        assert total['retired'] == total['arena_bytes'] - total['table_bytes']
    parsed[name] = (execs, owners, totals)

a, b = parsed['baseline'], parsed['candidate']
assert a[1].keys() == b[1].keys()
for label in labels:
    if not label.endswith('-500'):
        assert a[0][label] == b[0][label], ('nonempty execution changed', label)
    print('EXEC', label, 'baseline', a[0][label], 'candidate', b[0][label])
    print('TOTAL', label, 'baseline', a[2][label], 'candidate', b[2][label])
changed = 0
for key, before in a[1].items():
    after = b[1][key]
    if before != after:
        changed += 1
        # A cold empty rule may leave other nonempty inputs unforced. For all
        # other cases, only the empty active/parked occurrence should differ.
        assert key[0] in ['cold-500', 'warm-500', 'repeat-500'] or before[0] == after[0] == 0, key
        print('OWNER-CHANGE', key, 'baseline', before, 'candidate', after)
cold = b[2]['cold-500']
assert cold['table_bytes'] == cold['arena_bytes'] == cold['capacity_bytes'] == 0
assert b[0]['cold-500'][5] < a[0]['cold-500'][5]
print('PASS all 20 SQL windows and 78 ownership pairs reviewed;', changed, 'changed snapshots')
print('Nonempty requested allocations exactly unchanged; all warmed/repeated and second-rotation windows request zero.')
print('Payload capacity and requested layouts only: no RSS, peak live ownership, speed or Pi claim.')
