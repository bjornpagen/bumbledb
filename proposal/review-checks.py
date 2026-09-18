"""Small exact-rational checks for review-astra.md; no engine implementation.

Run: python3 proposal/review-checks.py
Uses exhaustive small models and Fraction, independently of Bumbledb.
"""
from fractions import Fraction as F
from itertools import product
import json
import random


def vertices(bounds):
    """Vertices of a box intersected with sum(p)=1: all but one at bounds."""
    n = len(bounds)
    out = set()
    for free in range(n):
        fixed = [i for i in range(n) if i != free]
        for chosen in product((0, 1), repeat=n - 1):
            p = [F(0)] * n
            for i, side in zip(fixed, chosen):
                p[i] = bounds[i][side]
            p[free] = 1 - sum(p)
            if bounds[free][0] <= p[free] <= bounds[free][1]:
                out.add(tuple(p))
    return sorted(out)


def event(bounds, mask):
    inside = [i for i in range(len(bounds)) if mask >> i & 1]
    outside = [i for i in range(len(bounds)) if not mask >> i & 1]
    return (max(sum(bounds[i][0] for i in inside), 1 - sum(bounds[i][1] for i in outside)),
            min(sum(bounds[i][1] for i in inside), 1 - sum(bounds[i][0] for i in outside)))


def expectation(bounds, values):
    order = sorted(range(len(bounds)), key=lambda i: (values[i], i))
    lo = hi = F(values[order[0]])
    for j in range(1, len(order)):
        step = values[order[j]] - values[order[j - 1]]
        mask = sum(1 << i for i in order[j:])
        lower, upper = event(bounds, mask)
        lo += step * lower
        hi += step * upper
    return lo, hi


grid = [F(i, 4) for i in range(5)]
tnorms = {
    "lukasiewicz": lambda a, b: max(F(0), a + b - 1),
    "product": lambda a, b: a * b,
    "minimum": min,
}
for name, mul in tnorms.items():
    for a, b, c in product(grid, repeat=3):
        assert mul(a, b) == mul(b, a)
        assert mul(mul(a, b), c) == mul(a, mul(b, c))
        assert mul(a, max(b, c)) == max(mul(a, b), mul(a, c))
        assert max(a, mul(a, b)) == a
        assert mul(a, 1) == a and mul(a, 0) == 0

a = b = c = F(1, 2)
bounded_sum = lambda x, y: min(F(1), x + y)
prob_sum = lambda x, y: x + y - x * y
assert min(a, bounded_sum(b, c)) == F(1, 2)
assert bounded_sum(min(a, b), min(a, c)) == 1
assert a * prob_sum(b, c) == F(3, 8)
assert prob_sum(a * b, a * c) == F(7, 16)

scale = 1 << 63
a, b, c = scale // 4, 5, 7 * scale // 8
rounded_mul = lambda x, y: x * y // scale
assert rounded_mul(rounded_mul(a, b), c) == 0
assert rounded_mul(a, rounded_mul(b, c)) == 1

# Fréchet conjunction is sound in every joint distribution, including
# correlated events and repeated leaves. Independent products are not
# universally sound lower bounds: disjoint half-probability events refute it.
assert tnorms["product"](F(1, 2), F(1, 2)) > 0
assert tnorms["minimum"](F(1, 2), F(1, 2)) > 0
gilio = (F(1, 2), F(3, 5), F(7, 10))
assert sum(gilio) - sum(tnorms["lukasiewicz"](gilio[i], gilio[j])
                        for i, j in ((0, 1), (0, 2), (1, 2))) == F(6, 5)
# Exact simultaneous pair minima are incoherent, but their lower-bound
# inequalities all hold in the independent model (as do the upper bounds).
for i, j in ((0, 1), (0, 2), (1, 2)):
    assert tnorms["lukasiewicz"](gilio[i], gilio[j]) <= gilio[i] * gilio[j] <= min(gilio[i], gilio[j])

rng = random.Random(731)
checked = 0
for _ in range(500):
    n = rng.choice((2, 3, 4))
    bounds = [tuple(sorted((F(rng.randrange(6), 5), F(rng.randrange(6), 5)))) for _ in range(n)]
    vs = vertices(bounds)
    feasible = sum(x[0] for x in bounds) <= 1 <= sum(x[1] for x in bounds)
    assert bool(vs) == feasible
    if not vs:
        continue
    tight = [(min(p[i] for p in vs), max(p[i] for p in vs)) for i in range(n)]
    assert tight == [event(bounds, 1 << i) for i in range(n)]
    assert vertices(tight) == vs
    masks = range(1 << n)
    for mask in masks:
        actual = [sum(p[i] for i in range(n) if mask >> i & 1) for p in vs]
        assert event(bounds, mask) == (min(actual), max(actual))
    for x, y in product(masks, repeat=2):
        assert event(bounds, x)[0] + event(bounds, y)[0] <= event(bounds, x | y)[0] + event(bounds, x & y)[0]
    vals = [rng.randrange(-5, 6) for _ in range(n)]
    actual = [sum(p[i] * vals[i] for i in range(n)) for p in vs]
    assert expectation(bounds, vals) == (min(actual), max(actual))
    checked += 1

demo = [(F(2, 5), F(3, 5)), (F(1, 5), F(2, 5)), (F(1, 10), F(3, 10))]
assert event(demo, 0b011) == (F(7, 10), F(9, 10))
assert expectation(demo, [0, 1, 2]) == (F(1, 2), F(9, 10))
assert expectation(demo, [10, 10, -30]) == (-2, 6)
refined = demo[:2] + [(F(1, 10), F(1, 10))]
assert event(refined, 0b011) == (F(9, 10), F(9, 10))
assert expectation(refined, [10, 10, -30]) == (6, 6)

# Identical singleton intervals can conceal a subset-mass equality.
segment_a = [(F(1, 2), F(1, 2), 0, 0), (0, 0, F(1, 2), F(1, 2))]
segment_b = [(F(1, 2), 0, F(1, 2), 0), (0, F(1, 2), 0, F(1, 2))]
box = lambda vs: [(min(p[i] for p in vs), max(p[i] for p in vs)) for i in range(4)]
assert box(segment_a) == box(segment_b)
assert {p[0] + p[1] for p in segment_a} == {0, 1}
assert {p[0] + p[1] for p in segment_b} == {F(1, 2)}

# Dependency-aware robust dominance, invisible to comparing output ranges.
vacuous = [(F(0), F(1)), (F(0), F(1))]
assert expectation(vacuous, [0, 100]) == (0, 100)
assert expectation(vacuous, [-1, 99]) == (-1, 99)
assert expectation(vacuous, [1, 1]) == (1, 1)

# A stronger dominance example: neither action dominates state by state.
bounded_binary = [(F(1, 5), F(4, 5))] * 2
assert expectation(bounded_binary, [10, 100]) == (28, 82)
assert expectation(bounded_binary, [0, 101]) == (F(101, 5), F(404, 5))
assert expectation(bounded_binary, [10, -1]) == (F(6, 5), F(39, 5))

# Conditioning can leave the box-simplex family. Its coordinate enclosure
# loses the ratio constraint q1 <= 2*q2, although each endpoint is tight.
prior = [(F(1, 10), F(1, 5))] * 3 + [(F(2, 5), F(7, 10))]
posterior_vertices = [tuple(p[i] / sum(p[:3]) for i in range(3)) for p in vertices(prior)]
posterior_box = [(min(q[i] for q in posterior_vertices), max(q[i] for q in posterior_vertices))
                 for i in range(3)]
assert posterior_box == [(F(1, 5), F(1, 2))] * 3
assert max(q[0] - 2*q[1] for q in posterior_vertices) == 0
assert expectation(posterior_box, [1, -2, 0])[1] == F(1, 10)

# Deterministic regrouping *is* closed: merged coordinates retain the exact
# total-mass lower/upper constraints of the disjoint source groups.
for groups in (((0, 1), (2,)), ((0,), (1, 2))):
    merged = [(sum(demo[i][0] for i in group), min(F(1), sum(demo[i][1] for i in group)))
              for group in groups]
    for values in ((-2, 3), (4, -1)):
        source_values = [0] * 3
        for group, value in zip(groups, values):
            for i in group:
                source_values[i] = value
        assert expectation(demo, source_values) == expectation(merged, values)

print(json.dumps({
    "rational_semiring_grid_checks": "passed: 3 algebras x 125 triples",
    "feasible_box_models_checked": checked,
    "box_checks": "vertex extrema, tightening, every event, 2-monotonicity, signed Choquet expectation",
    "rounded_product_associativity": {"scale": scale, "a": a, "b": b, "c": c, "left": 0, "right": 1},
    "decision_demo": "event [0.7,0.9], score [0.5,0.9], payoff [-2,6], refined payoff [6,6]",
    "same_box_different_model": "passed",
    "robust_dominance_with_overlapping_ranges": "passed, including crossing action payoffs",
    "conditioning_box_information_loss": "passed: exact upper 0, box upper 0.1",
    "deterministic_regrouping_closure": "passed"
}, indent=2))
