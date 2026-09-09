#!/usr/bin/env python3
"""Validate every current output/position owner and match prior triangle counts."""
import ast
import re
from diagnostics import Phase, ROUND, load

phase = Phase('q1-owners-review-1')
try:
    assert load(ROUND/'q1-owners-1/STATE.json')['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
    raw = (ROUND/'q1-owners-1/ownership.log').read_text()
    labels = re.findall(r'^PASS ((?:cold|warm|rotation0|rotation1)-(?:range|triangle)-[0-3])$', raw, re.M)
    assert len(labels) == len(set(labels)) == 32
    windows = dict(re.findall(r'EXEC (\S+) alloc=AllocWindow \{ ([^}]+) \}', raw))
    old_raw = (ROUND/'m1-saved-baseline-1/ownership.log').read_text()
    old_windows = dict(re.findall(r'EXEC (\S+) .*?alloc=AllocWindow \{ ([^}]+) \}', old_raw))
    answers = dict((m[0], tuple(map(int, m[1:]))) for m in re.findall(
        r'^ANSWER (\S+) rows=(\d+) arity=(\d+) cell_size=(\d+) cell_align=(\d+) cells=\((\d+), (\d+)\) cell_live_bytes=(\d+) cell_capacity_bytes=(\d+) text=\(0, 0\) blob=\(0, 0\)$', raw, re.M))
    sinks = dict((m[0], m[1:]) for m in re.findall(
        r'^SINK (\S+) dense=(.*?) hash_rows=(\d+) hash_owners=(\[.*?\]) hash_capacity_bytes=(\d+) scratch=(\(.*?\)) scan_rows=\(0, 0\) key_bytes=\(0, 0\) spilled=false$', raw, re.M))
    views = {}
    for label, occ, slot, rows, survivors, seed, filters in re.findall(
        r'^VIEW (\S+) occ=(\d+) slot=(\d+) image_rows=(\d+) survivors=(.*?) seed=(.*?) filters=(.*)$', raw, re.M):
        views.setdefault(label, []).append((int(occ), int(slot), int(rows), survivors, seed, filters))
    spares = {}
    for label, occ, length, cap in re.findall(r'^SPARE (\S+) occ=(\d+) len=(\d+) capacity=(\d+)$', raw, re.M):
        spares.setdefault(label, []).append((int(occ), int(length), int(cap)))
    totals = dict((label, int(total)) for label, total in re.findall(
        r'^SURVIVORS (\S+) all_active_parked_spare_capacity_bytes=(\d+)$', raw, re.M))
    assert set(labels) == windows.keys() == answers.keys() == sinks.keys() == views.keys() == spares.keys() == totals.keys()
    checked_views = 0
    for label in labels:
        kind, family, draw = label.split('-')
        draw = int(draw)
        a = answers[label]
        rows = 2000 if family == 'range' else (0 if draw == 3 else 5)
        arity = 2 if family == 'range' else 1
        cap = rows * arity if family == 'range' or kind in ('cold', 'warm') else 5
        assert a == (rows, arity, 24, 8, rows*arity, cap, rows*arity*24, cap*24), (label, a)
        dense, hash_rows, owners, hash_bytes, scratch = sinks[label]
        owners = ast.literal_eval(owners)
        assert len(owners) == 5 and all(length <= capacity for length, capacity, _ in owners)
        assert sum(capacity*size for _, capacity, size in owners) == int(hash_bytes)
        assert ast.literal_eval(scratch) == (arity, arity)
        if family == 'range':
            assert dense == 'Some((2000, 4000, 4096))' and int(hash_rows) == int(hash_bytes) == 0
        else:
            assert dense == 'None' and int(hash_rows) == rows
            dense_capacity = 0 if kind in ('cold', 'warm') and draw == 3 else 8
            assert int(hash_bytes) == 83886087 + dense_capacity * 4
            assert [entry[:2] for entry in owners[:2]] == [(8388615, 8388615), (8388608, 8388608)]
            assert owners[2][2] == 0  # Vec<()> has usize::MAX capacity, zero payload bytes.
            old_label = f'{kind}-{[1,167,333,500][draw]}'
            assert windows[label].strip() == old_windows[old_label].strip(), label
        slots = 1 if kind in ('cold', 'warm') else (draw+1 if kind == 'rotation0' else 4)
        assert len(views[label]) == slots + (2 if family == 'triangle' else 0)
        assert len(spares[label]) == (1 if family == 'range' else 3)
        assert all(length == cap == 0 for _, length, cap in spares[label])
        total = 0
        for occ, slot, image_rows, survivors, seed, filters in views[label]:
            checked_views += 1
            assert image_rows == 100000
            if occ:
                assert survivors == seed == 'None' and filters == '[]' and slot == 0
            else:
                assert survivors.startswith('Some(') and seed.startswith('Some(')
                length, capacity = ast.literal_eval(survivors[5:-1])
                pivot, seed_len, seed_cap = ast.literal_eval(seed[5:-1])
                assert pivot == 0 and capacity == seed_cap == 100000
                assert length <= seed_len
                if family == 'range':
                    assert length == 2000 and seed_len in (93750,81250,68750,56250)
                else:
                    assert (length,seed_len) in ((529,49508),(495,32937),(500,16517),(0,0))
                total += 4*capacity
        assert total == totals[label] == slots * 400000
        if kind in ('warm','rotation1'):
            assert all(int(v) == 0 for v in re.findall(r': (\d+)', windows[label]))
        print(f'CHECK {label} answers={rows} cells_retained={a[-1]} hash_retained={hash_bytes} survivor_retained={total} view_owners={len(views[label])}')
    assert checked_views == 100
    phase.state.update(windows=32, views=checked_views, answer_owners=32, sink_owners=32,
        spare_owners=sum(map(len,spares.values())), prior_triangle_windows_matched=16,
        limitations='Payload capacities and requested allocator layouts, not RSS, peak, elapsed time or Pi qualification')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('ALL-OWNERS-AND-PRIOR-TRIANGLE-COUNTS-VERIFIED')
