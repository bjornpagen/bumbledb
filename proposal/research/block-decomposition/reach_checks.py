"""Independent finite check of a backward adaptation of ReachBdd Algorithm 1.

This enumerates relations, not decision-diagram encodings. It verifies the block
fixed-point equations before an eventual Rust implementation. It is not a timing
comparison with Sylvan or the paper's implementation.
"""
from pathlib import Path
import functools, hashlib, json

HERE = Path(__file__).resolve().parent

def may(relation, goal):
    return sum((bool(row & goal) << i) for i, row in enumerate(relation))

def bfs(relation, goal):
    for _ in range(len(relation) + 1):
        next_goal = goal | may(relation, goal)
        if next_goal == goal: return goal
        goal = next_goal
    raise AssertionError('finite reachability did not stabilize')

@functools.lru_cache(None)
def tiny(relation, goal):
    return reach(relation, goal)

def reach(relation, goal):
    size = len(relation)
    full = (1 << size) - 1
    if goal in (0, full) or not any(relation): return goal
    if all(row == full for row in relation): return full
    assert size > 1 and size & (size - 1) == 0
    half = size // 2
    mask = (1 << half) - 1
    r00 = tuple(row & mask for row in relation[:half])
    r01 = tuple(row >> half for row in relation[:half])
    r10 = tuple(row & mask for row in relation[half:])
    r11 = tuple(row >> half for row in relation[half:])
    g0, g1 = goal & mask, goal >> half
    recurse = tiny if half <= 2 else reach
    for _ in range(size + 1):
        old = g0, g1
        g0 = recurse(r00, g0)
        g1 |= may(r10, g0)
        g1 = recurse(r11, g1)
        g0 |= may(r01, g1)
        if old == (g0, g1): return g0 | (g1 << half)
    raise AssertionError('block reachability did not stabilize')

cases = 0
for size in [1, 2, 4]:
    goals = range(1 << size) if size < 4 else [0, 15, 1, 2, 4, 8]
    mask = (1 << size) - 1
    for encoded in range(1 << (size * size)):
        relation = tuple((encoded >> (i * size)) & mask for i in range(size))
        for goal in goals:
            assert reach(relation, goal) == bfs(relation, goal), (relation, goal)
            cases += 1
# Multiple goals and larger dead-ended chains supplement exhaustive small graphs.
for size in [4, 8, 16, 64]:
    relation = tuple(1 << (i + 1) if i + 1 < size else 0 for i in range(size))
    for goal in [0, 1, 1 << (size - 1), (1 << size) - 1, sum(1 << i for i in range(0, size, 3))]:
        assert reach(relation, goal) == bfs(relation, goal)
        cases += 1
result = {'passed': True, 'cases': cases,
          'scope': 'All one/two-state relations and goals; all four-state relations with empty/full/singleton goals; larger chain cases. Finite semantic check, not a performance measurement.',
          'checker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          'paper': '2212.03684v1',
          'paper_sha256': hashlib.sha256((HERE/'decision-diagram-reachability.pdf').read_bytes()).hexdigest()}
(HERE/'reach-checks.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
