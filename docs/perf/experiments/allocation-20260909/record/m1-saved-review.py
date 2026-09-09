#!/usr/bin/env python3
"""Review all saved-map windows, retaining exact equality and negative evidence."""
import re
from diagnostics import ROUND, load

paths = {name: ROUND/f'm1-saved-{name}-{attempt}'
         for name, attempt in [('baseline', 1), ('duplicate', 2)]}
states = {name: load(path/'STATE.json') for name, path in paths.items()}
for state in states.values():
    assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
    assert all(step['status'] == 'PASS' for step in state['steps'])
for field in ['dependencies', 'database_sha256', 'oracle_sha256']:
    assert states['baseline'][field] == states['duplicate'][field]
logs = {name: (path/'ownership.log').read_text() for name, path in paths.items()}
clean = lambda text: re.sub(r'finished in [0-9.]+s', 'finished in <diagnostic time>', text)
assert clean(logs['baseline']) == clean(logs['duplicate']), 'review every changed row'

labels = [f'{kind}-{key}' for key in [1, 167, 333, 500] for kind in ['cold', 'warm', 'repeat']]
labels += [f'rotation{cycle}-{key}' for cycle in range(2) for key in [1, 167, 333, 500]]
for name, content in logs.items():
    windows = re.findall(r'EXEC ([^ ]+) lo=(\d+) hi=(\d+) answers=(\d+) '
        r'alloc=AllocWindow \{ allocs: (\d+), deallocs: (\d+), alloc_bytes: (\d+), dealloc_bytes: (\d+) \} '
        r'clones=\((\d+), (\d+), (\d+)\)', content)
    assert [row[0] for row in windows] == labels
    for row in windows:
        label = row[0]
        values = list(map(int, row[1:]))
        assert values[2] == (0 if values[0] == 500 else 5)
        assert values[7:] == [0, 0, 0]
        if label.startswith(('warm-', 'repeat-', 'rotation1-')):
            assert values[3:7] == [0, 0, 0, 0]
    totals = re.findall(r'^TOTAL (.+)$', content, re.MULTILINE)
    owners = re.findall(r'^COLT (.+)$', content, re.MULTILINE)
    assert len(totals) == 20 and len(owners) == 78
    print(name, 'reviewed', len(windows), 'exact SQL windows,', len(owners),
          'active/parked snapshots; all clone counters zero; 8 warm/repeat + 4 second-rotation windows zero allocation')
    for total in totals:
        print('TOTAL', name, total)
print('PASS: baseline/candidate requested allocations and all actual ownership rows are identical.')
print('Do not attribute the ordinary cold timing signal to allocation reduction in this workload.')
