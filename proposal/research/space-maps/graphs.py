"""Finite relation graphs unify maps, partitions, images and information cells."""
from pathlib import Path
import hashlib
import itertools
import json

def compose(a, b):
    return {(x, z) for x, y in a for v, z in b if y == v}

def converse(r):
    return {(b, a) for a, b in r}

def may(r, event):
    return {a for a, b in r if b in event}

def subsets(n):
    return [{i for i in range(n) if mask >> i & 1} for mask in range(1 << n)]

relations = homomorphisms = events = 0
T, S = set(range(3)), set(range(2))
IdS, IdT = {(i, i) for i in S}, {(i, i) for i in T}
for mask in range(1 << 6):
    G = {(t, s) for t in T for s in S if mask >> (2*t+s) & 1}
    total = {t for t, _ in G} == T
    functional = compose(converse(G), G) <= IdS
    boolean = may(G, S) == T
    for E in subsets(2):
        boolean &= may(G, S-E) == T-may(G, E)
        for F in subsets(2):
            boolean &= may(G, E & F) == may(G, E) & may(G, F)
        events += 1
    assert boolean == (total and functional)
    if boolean:
        images = [frozenset(may(G, E)) for E in subsets(2)]
        assert (len(set(images)) == 4) == ({s for _, s in G} == S)
        K = compose(G, converse(G))
        assert IdT <= K and converse(K) == K and compose(K, K) <= K
        for E in subsets(3):
            # Image then inverse image gives the least union of observation cells.
            image = may(converse(G), E)
            saturated = may(G, image)
            assert saturated == may(K, E) and E <= saturated
            assert may(K, saturated) == saturated
            for A in subsets(2):
                assert (image <= A) == (E <= may(G, A))
        homomorphisms += 1
    relations += 1

result = {
    'passed': True,
    'relations': relations,
    'total_function_graphs': homomorphisms,
    'event_complement_checks': events,
    'checker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    'scope': 'Independent finite mathematics. Does not implement a general graph-backed CoordinateMap or law transport.',
}
Path(__file__).with_suffix('.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
