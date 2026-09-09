#!/usr/bin/env python3
"""Review all saved SQL windows and all four rehash owners in three variants."""
import argparse
import ast
import re
import shutil
from diagnostics import ROUND, Phase, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
saved_paths = {v: ROUND/f'g2-saved-{v}-{2 if v == "g2" else 1}' for v in ['baseline','g1','g2']}
phase = Phase(f'g2-saved-review-{args.attempt}')
shutil.copy2(__file__, phase.path/'review.py')
review_log = (phase.path/'review.log').open('x')
def report(*parts):
    print(*parts, file=review_log, flush=True)
labels = [f'{kind}-{key}' for key in [1, 167, 333, 500] for kind in ['cold', 'warm', 'repeat']]
labels += [f'rotation{cycle}-{key}' for cycle in range(2) for key in [1, 167, 333, 500]]

def parse(path, scratch=True):
    state = load(path/'STATE.json')
    assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
    assert all(s['status'] == 'PASS' for s in state['steps'])
    content = (path/'ownership.log').read_text()
    executions = re.findall(r'EXEC ([^ ]+) lo=(\d+) hi=(\d+) answers=(\d+) '
        r'alloc=AllocWindow \{ allocs: (\d+), deallocs: (\d+), alloc_bytes: (\d+), dealloc_bytes: (\d+) \} '
        r'clones=\((\d+), (\d+), (\d+)\)', content)
    assert [r[0] for r in executions] == labels
    execs = {r[0]: tuple(map(int, r[1:])) for r in executions}
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
        snaps = [s for (window, _, _), s in owners.items() if window == label]
        total = totals[label]
        assert total['colts'] == len(snaps)
        for field, i in [('table_bytes',3),('arena_bytes',4),('capacity_bytes',5),('retained',6)]:
            assert total[field] == sum(s[i] for s in snaps)
        assert total['retired'] == total['arena_bytes'] - total['table_bytes']
    scratches = {}
    if scratch:
        for label, occ, slot, length, capacity, size in re.findall(
            r'^SCRATCH ([^ ]+) occ=(\d+) slot=(\d+) len=(\d+) capacity=(\d+) bytes=(\d+)$', content, re.MULTILINE):
            key = (label, int(occ), int(slot))
            assert key not in scratches
            value = tuple(map(int,(length, capacity, size)))
            assert value[0] <= value[1] and value[2] == value[1] * 8
            scratches[key] = value
        assert scratches.keys() == owners.keys()
    return state, execs, owners, totals, scratches

try:
    results = {v: parse(path) for v, path in saved_paths.items()}
    for v in ['g1','g2']:
        for field in ['dependencies','database_sha256','oracle_sha256','protocol']:
            assert results[v][0][field] == results['baseline'][0][field]
    # Adding scratch observation must not change ANY previous execution or
    # arena/pool snapshot. Preserve the independently measured old evidence.
    for v, old in [('baseline','m1-saved-baseline-1'),('g1','g1-saved-1')]:
        previous = parse(ROUND/old, scratch=False)
        assert results[v][1:4] == previous[1:4]
    a, g1, b = [results[v] for v in ['baseline','g1','g2']]
    assert a[2].keys() == g1[2].keys() == b[2].keys()
    changed = 0
    for key, before in a[2].items():
        after = b[2][key]
        assert before[:2] == after[:2]
        assert before[3] == after[3] and before[7] == after[7]
        assert after[4] == after[3], ('retired map content', key)
        assert b[4][key][0] == 0, ('used scratch escaped', key)
        for length, capacity in after[2]:
            assert length <= capacity
        # All other COLT owners must be identical: account explicitly for the
        # larger scratch, not just smaller map-arena capacities.
        others = [r[2][key][6] - r[2][key][5] - r[4][key][2] for r in [a,g1,b]]
        assert others[0] == others[1] == others[2], ('other owners changed', key, others)
        if before != after or a[4][key] != b[4][key]:
            changed += 1
            report('OWNER', key, {v: {'snapshot':r[2][key], 'scratch':r[4][key],
                'four_owner_capacity':r[2][key][5]+r[4][key][2]} for v,r in results.items()})
    for label in labels:
        assert a[1][label][:3] == g1[1][label][:3] == b[1][label][:3]
        report('EXEC', label, {v:r[1][label] for v,r in results.items()})
        report('TOTAL', label, {v:r[3][label] for v,r in results.items()})
    root = ('cold-1',1,0)
    assert b[2][root][2] == [(131072,131072),(262144,262144),(43236,65536)]
    assert b[4][root] == (0,52428,419424)
    assert b[2][root][5] + b[4][root][2] == 2909792
    phase.state.update(changed_snapshots=changed, windows=20, owners=78, variants=3,
        input_hashes={v:digest(saved_paths[v]/'ownership.log') for v in results},
        conclusion='All SQL answers/map shapes match. All retired G2 arena contents disappear. '
        'Scratch included; other owners unchanged. Prior baseline/G1 results reproduced exactly. '
        'Requested layouts/payload capacity only: not RSS, peak, speed, mmap or Pi qualification.')
    phase.save()
    report('PASS all 20 windows and 78 owners in all three variants, including scratch.')
    print('PASS all 20 windows and 78 owners in all three variants, including scratch.', flush=True)
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('ALL-OWNERS-AND-SCRATCH-REVIEWED')
finally:
    review_log.close()
