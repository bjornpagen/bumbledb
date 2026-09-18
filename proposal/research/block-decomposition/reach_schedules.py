"""Stress the backward block schedule with the paper's ideal and adverse families.

Explicit finite relations and deterministic operation counts, not Rust or timing.
The alternating leading bit makes both diagonal blocks empty. The lower counter
still advances monotonically. This detects a fixture bias before native Reach
benchmarks select an implementation using only the favorable counter family.
"""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent

def preimage(relation, goal):
    return sum(bool(row & goal) << i for i, row in enumerate(relation))

def ordinary(relation, goal):
    calls = 0
    while True:
        calls += 1
        next_goal = goal | preimage(relation, goal)
        if next_goal == goal:
            return goal, calls
        goal = next_goal

def recursive(relation, goal):
    memo = {}
    counters = dict(nonterminal_calls=0, cache_hits=0, block_loops=0, preimages=0)

    def reach(r, g):
        full = (1 << len(r)) - 1
        if g in (0, full) or not any(r):
            return g
        if all(row == full for row in r):
            return full
        key = (r, g)
        if key in memo:
            counters['cache_hits'] += 1
            return memo[key]
        counters['nonterminal_calls'] += 1
        half = len(r) // 2
        assert half > 0 and len(r) == 2*half
        mask = (1 << half) - 1
        r00, r01 = tuple(row & mask for row in r[:half]), tuple(row >> half for row in r[:half])
        r10, r11 = tuple(row & mask for row in r[half:]), tuple(row >> half for row in r[half:])
        g0, g1 = g & mask, g >> half
        while True:
            counters['block_loops'] += 1
            counters['preimages'] += 2
            old = (g0, g1)
            g0 = reach(r00, g0)
            g1 |= preimage(r10, g0)
            g1 = reach(r11, g1)
            g0 |= preimage(r01, g1)
            if old == (g0, g1):
                answer = g0 | (g1 << half)
                memo[key] = answer
                return answer
    answer = reach(relation, goal)
    return answer, counters

cases = []
for width in range(1, 9):
    n = 1 << width
    counter = tuple(1 << (x+1) if x+1 < n else 0 for x in range(n))
    alternating = tuple(1 << ((1-b)*n+x+1) if x+1 < n else 0
                        for b in [0, 1] for x in range(n))
    fixtures = [('counter', counter, 1 << (n-1)),
                ('alternating-leading-bit', alternating, (1 << (n-1)) | (1 << (2*n-1)))]
    for name, relation, goal in fixtures:
        reference, calls = ordinary(relation, goal)
        result, counts = recursive(relation, goal)
        assert result == reference == (1 << len(relation))-1
        cases.append(dict(family=name, counter_bits=width, states=len(relation),
                          ordinary_preimages=calls, recursive=counts))

result = dict(passed=True, cases=cases,
    scope='Explicit finite relations, same full reachable-state answer. Counts of semantic schedule operations, not graph visits or native timing.',
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    paper='2212.03684v1',
    paper_sha256=hashlib.sha256((HERE/'decision-diagram-reachability.pdf').read_bytes()).hexdigest())
(HERE/'reach-schedules.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
