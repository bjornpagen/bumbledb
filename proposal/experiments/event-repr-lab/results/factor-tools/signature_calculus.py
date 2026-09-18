"""Derive the scope-independent pair table and check its exact finite boundary.

This is qualitative composition between Event values, not relational product
between state coordinates. No timings or native performance claims are made.
"""
from itertools import product
from pathlib import Path
import hashlib
import json

LAB = Path(__file__).resolve().parent
ROOT = LAB.parents[2]


def projection(triple, left, right):
    return sum(1 << cell for cell in {
        (2 * ((world >> left) & 1) + ((world >> right) & 1))
        for world in range(8) if triple >> world & 1
    })


def demand(triple):
    # Each A,C cell can be assigned to B=false, B=true, or both.
    return tuple(sum((triple >> (4*a + 2*b + c)) & 1 for b in range(2))
                 for a in range(2) for c in range(2))


TRIPLES = [(t, projection(t, 2, 1), projection(t, 1, 0),
            projection(t, 2, 0), demand(t)) for t in range(1, 256)]
TABLE = [[0]*16 for _ in range(16)]
MIN_WORLDS = {}
for t, ab, bc, ac, _ in TRIPLES:
    TABLE[ab][bc] |= 1 << ac
    key = ab, bc, ac
    MIN_WORLDS[key] = min(MIN_WORLDS.get(key, 9), bin(t).count('1'))


def compose(left, right):
    out = 0
    for a in range(1, 16):
        for b in range(1, 16):
            if left >> a & 1 and right >> b & 1:
                out |= TABLE[a][b]
    return out


def converse(sig):
    return (sig & 9) | ((sig & 2) << 1) | ((sig & 4) >> 1)


def converse_mask(mask):
    return sum(1 << converse(sig) for sig in range(1, 16) if mask >> sig & 1)


def signature(a, b, size):
    out = 0
    for w in range(size):
        out |= 1 << (2 * ((a >> w) & 1) + ((b >> w) & 1))
    return out


def abstract_pairs(counts):
    return {(ab, bc) for _, ab, bc, _, needed in TRIPLES
            if all((have > 0) == (need > 0) and have >= need
                   for have, need in zip(counts, needed))}


def concrete_pairs(a, c, size):
    return {(signature(a, b, size), signature(b, c, size))
            for b in range(1 << size)}


def main():
    # The signature is sufficient for emptiness/fullness of every binary
    # Boolean expression, and each of its bits is necessary (use a minterm).
    boolean_cases = 0
    for sig in range(1, 16):
        cells = [cell for cell in range(4) if sig >> cell & 1]
        for op in range(16):
            values = [(op >> cell) & 1 for cell in cells]
            assert any(values) == bool(op & sig)
            assert all(values) == (sig & (op ^ 15) == 0)
            boolean_cases += 1
    identity = (1 << 1) | (1 << 8) | (1 << 9)
    for a in range(1, 16):
        assert compose(1 << a, identity) == compose(identity, 1 << a) == 1 << a
        for b in range(1, 16):
            assert converse_mask(TABLE[a][b]) == TABLE[converse(b)][converse(a)]
            for c in range(1, 16):
                assert compose(TABLE[a][b], 1 << c) == compose(1 << a, TABLE[b][c])

    # All labelled powerset Events on one through five worlds. This compares
    # concrete B enumeration against a small independent capacity calculation.
    finite_cases = 0
    for size in range(1, 6):
        for a in range(1 << size):
            for c in range(1 << size):
                counts = [0]*4
                for w in range(size):
                    counts[2*((a >> w)&1) + ((c >> w)&1)] += 1
                actual = concrete_pairs(a, c, size)
                assert abstract_pairs(counts) == actual
                assert abstract_pairs([min(n, 2) for n in counts]) == actual
                ac = signature(a, c, size)
                assert all(TABLE[ab][bc] >> ac & 1 for ab, bc in actual)
                finite_cases += 1

    # Representative cells of size 0,1,2,3 test the saturation threshold and
    # all four cells being simultaneously split. These can need twelve worlds.
    capacity_cases = assignments = 0
    for counts in product(range(4), repeat=4):
        if not any(counts):
            continue
        a = c = size = 0
        for cell, count in enumerate(counts):
            for _ in range(count):
                a |= (cell >> 1) << size
                c |= (cell & 1) << size
                size += 1
        actual = concrete_pairs(a, c, size)
        assert abstract_pairs(counts) == actual
        assert abstract_pairs([min(n, 2) for n in counts]) == actual
        capacity_cases += 1
        assignments += 1 << size

    # Same scope and same exact pair signature, different composition answer.
    # A=C={0} has no proper nonempty subset; A=C={0,1} has one.
    assert signature(1, 1, 3) == signature(3, 3, 3) == 9
    assert (13, 11) not in concrete_pairs(1, 1, 3)
    assert (13, 11) in concrete_pairs(3, 3, 3)
    assert TABLE[13][11] >> 9 & 1

    # A-closure is not global satisfiability: four pairwise disjoint nonempty
    # proper Events cannot fit in three worlds, though all triangle constraints
    # are realizable and the fifteen-class network is already closed.
    network = [[1 << (9 if i == j else 7) for j in range(4)] for i in range(4)]
    for i, j, k in product(range(4), repeat=3):
        assert network[i][j] & compose(network[i][k], network[k][j]) == network[i][j]
    assert not any(all(signature(values[i], values[j], 3) == 7
                       for i in range(4) for j in range(i+1, 4))
                   for values in product(range(1, 7), repeat=4))
    assert all(signature(1 << i, 1 << j, 3) == 7
               for i in range(3) for j in range(i+1, 3))

    paper = ROOT/'proposal/papers/dylla-mossakowski-schneider-wolter-2013-algebraic-qualitative-calculi.pdf'
    reading = ROOT/'proposal/review-evidence/dylla-mossakowski-schneider-wolter-2013-algebraic-qualitative-calculi.txt'
    out = dict(
        passed=True,
        subject='Qualitative composition over Event values; not state relational product',
        table=TABLE,
        binary_operation_signature_cases=boolean_cases,
        triple_occupancies=255,
        realizable_signature_triples=len(MIN_WORLDS),
        largest_minimum_witness_worlds=max(MIN_WORLDS.values()),
        associativity_atom_triples=15**3,
        identity_signatures=[1, 8, 9],
        finite_endpoint_cases=finite_cases,
        saturated_capacity_cases=capacity_cases,
        capacity_assignment_enumerations=assignments,
        finite_boundary_counterexample=dict(support=[0, 1, 2], signature=9,
            requested_ab=13, requested_bc=11, impossible_a_c=[0], possible_a_c=[0, 1]),
        closure_counterexample=dict(worlds=3, unknown_events=4,
            off_diagonal_signature=7, diagonal_signature=9,
            algebraically_closed=True, every_triangle_realizable=True,
            globally_realizable=False),
        theorem_scope='Full finite powerset algebra, one existential intermediate Event; four cell cardinalities capped at two suffice. This refinement is not claimed closed under repeated composition.',
        checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        reading=dict(arxiv='1305.7345v2', sections='Definitions 2–3, Fact 4, section 2.2 and section 3.2',
            files={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
                   for p in [paper, reading]}))
    path = LAB/'results/signature-calculus-check.json'
    path.write_text(json.dumps(out, indent=2)+'\n')
    print(json.dumps({k:v for k,v in out.items() if k not in ['table', 'reading']}, indent=2))


if __name__ == '__main__':
    main()
