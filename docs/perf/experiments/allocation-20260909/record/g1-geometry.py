#!/usr/bin/env python3
"""Arithmetic growth model, calibrated to an observed saved-corpus root.

Not an allocator run, timing, peak memory or candidate acceptance.
"""
import ast
import json
import re
import shutil
from diagnostics import Phase, ROUND, digest

phase = Phase('g1-geometry-1')
shutil.copy2(__file__, phase.path/'geometry.py')
source = ROUND/'m1-saved-baseline-1/ownership.log'
line = next(row for row in source.read_text().splitlines()
            if row.startswith('COLT cold-1 occ=1 slot=0 '))
owners = ast.literal_eval(re.search(r'owners=(\[.*?\])', line)[1])
histogram = ast.literal_eval(re.search(r'histogram=(.*)$', line)[1])
assert histogram == {(1, 16384, 43236): 1}
assert 'rows=100000 maps=1 ' in line

class Pool:
    def __init__(self, width):
        self.width, self.length, self.capacity = width, 0, 0
        self.requests = []
    def reserve(self, needed):
        if needed > self.capacity:
            self.capacity = max(needed, 2*self.capacity, 8)
            self.requests.append(self.width*self.capacity)
    def resize(self, length):
        self.reserve(length)
        self.length = length

def model(mode):
    ctrl, buckets, dense = pools = [Pool(n) for n in (1,8,4)]
    rows, keys, stride = 100000, 43236, 16
    guess = min(max(rows//8, 16), max(rows, 1)*2)
    groups = 1 << (max(guess*5//16, 1)-1).bit_length()
    assert groups == 4096
    ctrl.resize(groups*8)
    buckets.resize(groups*stride)
    growths, copied = [], 0
    for length in range(keys):
        if (length+1)*5 > groups*16:
            groups *= 2
            for pool, extra in zip(pools, (groups*8, groups*stride, length)):
                pool.resize(pool.length+extra)
            growths.append((groups, length))
            if mode == 'compact-each-growth':
                for pool, live in zip(pools, (groups*8, groups*stride, length)):
                    copied += pool.width*live
                    pool.length = live
        dense.resize(dense.length+1)
    if mode == 'compact-at-finish':
        for pool, live in zip(pools, (groups*8, groups*stride, keys)):
            copied += pool.width*live
            pool.length = live
    result = dict(mode=mode, owners=[(p.length,p.capacity) for p in pools],
        arena_bytes=sum(p.width*p.length for p in pools),
        capacity_bytes=sum(p.width*p.capacity for p in pools),
        requests=sum(len(p.requests) for p in pools),
        requested_bytes=sum(sum(p.requests) for p in pools),
        copied_bytes=copied, growths=growths)
    if mode == 'append':
        assert result['owners'] == owners
        assert result['arena_bytes'] == 4229620
        assert result['capacity_bytes'] == 4423680
    return result

try:
    results = [model(mode) for mode in ('append','compact-at-finish','compact-each-growth')]
    phase.state.update(baseline_log_sha256=digest(source), baseline_owner_line=line,
        results=results, limitations='Arithmetic model only; append geometry exactly matches '
        'saved root lengths/capacities. It excludes other COLT pools, allocator usable size, '
        'RSS, transients, timing and real candidate costs. Growth copy work requires validation.')
    phase.save()
    print(json.dumps(results, indent=2))
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('MODEL-CALIBRATED-NOT-MEASURED-CANDIDATE')
