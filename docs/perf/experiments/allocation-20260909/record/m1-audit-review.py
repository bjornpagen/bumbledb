#!/usr/bin/env python3
"""Compare every baseline/candidate construction cell, including unchanged controls."""
import ast
import re
from diagnostics import ROUND, load

records = {}
pattern = (r'MAP width=(\d+) distinct=(\d+) grouped=(false|true) owners=(\[.*?\]) '
           r'clone_owners=(\[.*?\]) cold=(\(.*?\)) clone=(\(.*?\)) reused=(\(.*?\))')
for role, name in [('baseline', 'm1-audit-baseline-2'), ('candidate', 'm1-audit-duplicate-1')]:
    path = ROUND/name
    state = load(path/'STATE.json')
    assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
    rows = {}
    for width, distinct, grouped, *fields in re.findall(pattern, (path/'ownership.log').read_text()):
        key = (int(width), int(distinct), grouped == 'true')
        assert key not in rows
        rows[key] = dict(zip(['owners', 'clone_owners', 'cold', 'clone', 'reused'], map(ast.literal_eval, fields)))
        assert rows[key]['reused'] == (0, 0, 0, 0)
    assert set(rows) == {(w, d, g) for w in [0, 1, 2, 3, 4, 5, 8] for d in [25, 26] for g in [False, True]}
    records[role] = rows

print('| Width | Distinct | Grouped | Table groups A/B | Three-arena live bytes A/B | All-pool retained A/B | Cold requests A/B | Cold requested bytes A/B | Clone requested bytes A/B |')
print('|---|---|---|---|---|---|---|---|---|')
controls = 0
for key, before in records['baseline'].items():
    after = records['candidate'][key]
    if key[0] == 0 or key[1] == 26:
        assert before == after, (key, before, after)
        controls += 1
    else:
        assert before['owners'][0] == 16 and after['owners'][0] == 8
        assert after['owners'][8] < before['owners'][8]
        assert after['owners'][9] < before['owners'][9]
        assert after['cold'][0] < before['cold'][0]
        assert after['cold'][2] < before['cold'][2]
        assert after['clone'][2] < before['clone'][2]
    pairs = [(before['owners'][i], after['owners'][i]) for i in [0, 8, 9]]
    pairs += [(before[field][i], after[field][i]) for field, i in [('cold', 0), ('cold', 2), ('clone', 2)]]
    print('|', ' | '.join(map(str, key)), '|', ' | '.join(f'{a}/{b}' for a, b in pairs), '|')
print(f'{controls} unchanged control cells; all 28 cells have zero-request reset/reforce.')
print('Synthetic cases; requested layouts/capacities, not RSS, whole-query peak or timing.')
