#!/usr/bin/env python3
"""Count all four relevant owners for an occupied-tuple staging hypothesis.

Arithmetic only. G1/baseline three-arena geometry is calibrated to saved
measurements; prospective staging has not been implemented or measured.
"""
import json
import shutil
from diagnostics import Phase, ROUND, load

phase = Phase('g2-geometry-1')
shutil.copy2(__file__, phase.path/'geometry.py')
calibration = {r['mode']: r for r in load(ROUND/'g1-geometry-1/STATE.json')['results']}

class Pool:
    def __init__(self, width):
        self.width, self.length, self.capacity = width, 0, 0
        self.requests = []
    def resize(self, length):
        if length > self.capacity:
            self.capacity = max(length, 2*self.capacity, 8)
            self.requests.append(self.width*self.capacity)
        self.length = length

def model(mode):
    ctrl, buckets, dense, scratch = [Pool(width) for width in (1,8,4,8)]
    arenas = [ctrl, buckets, dense]
    pools = [*arenas, scratch]
    groups, arity, keys = 4096, 1, 43236
    stride = 8*(arity+1)
    ctrl.resize(groups*8)
    buckets.resize(groups*stride)
    relocation, scratch_writes, growths = 0, 0, []
    for length in range(keys):
        if (length+1)*5 > groups*16:
            groups *= 2
            growths.append((groups, length))
            if mode == 'staged-live':
                scratch.resize(length*(arity+1))
                scratch_writes += scratch.length*8
                ctrl.resize(groups*8)
                buckets.resize(groups*stride)
                # Rewrite the same dense entries in place; do not append.
                scratch.length = 0
            else:
                scratch.resize(arity)
                scratch_writes += length*arity*8
                for pool, extra in zip(arenas, (groups*8,groups*stride,length)):
                    pool.resize(pool.length+extra)
                if mode == 'compact-each-growth':
                    for pool, live in zip(arenas, (groups*8,groups*stride,length)):
                        relocation += pool.width*live
                        pool.length = live
        dense.resize(dense.length+1)
    result = dict(mode=mode, arena_owners=[(p.length,p.capacity) for p in arenas],
        scratch_owner=(scratch.length,scratch.capacity),
        arena_capacity_bytes=sum(p.width*p.capacity for p in arenas),
        scratch_capacity_bytes=scratch.width*scratch.capacity,
        all_four_capacity_bytes=sum(p.width*p.capacity for p in pools),
        all_four_requests=sum(len(p.requests) for p in pools),
        all_four_requested_bytes=sum(sum(p.requests) for p in pools),
        whole_table_relocation_bytes=relocation, scratch_payload_write_bytes=scratch_writes,
        growths=growths)
    if mode in calibration:
        old = calibration[mode]
        assert result['arena_owners'] == [tuple(pair) for pair in old['owners']]
        assert result['arena_capacity_bytes'] == old['capacity_bytes']
        assert sum(len(p.requests) for p in arenas) == old['requests']
        assert sum(sum(p.requests) for p in arenas) == old['requested_bytes']
        assert relocation == old['copied_bytes']
    return result

try:
    results = [model(mode) for mode in ('append','compact-each-growth','staged-live')]
    phase.state.update(results=results,
        scope='One saved 100000-row/43236-key width-one root; identical bucket/load schedule. '
        'All three arenas PLUS the existing rehash scratch, including retained scratch capacity. '
        'Other owners, allocator usable size, RSS, peak live bytes and CPU time are excluded. '
        'Scratch writes and whole-table relocation are separate operations, not total memory traffic.',
        acceptance='Arithmetic hypothesis, not an implemented or measured G2 candidate.')
    phase.save()
    print(json.dumps(results, indent=2))
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('FOUR-OWNER-MODEL-COMPLETE-NOT-A-CANDIDATE-MEASUREMENT')
