"""Exact, tiny-model falsifiers for proposal.md and laws.md; no engine code.

Run from the repository: python3 proposal/spec-checks.py
Only Python's standard library is used. H-polytopes must be bounded.
General vector routines support finite categorical frames; scoped examples
use distinct binary axes. Enumeration is intentionally unsuitable for scale.
"""

from dataclasses import dataclass
from fractions import Fraction as F
from itertools import combinations, permutations, product
import json


def vec(*xs):
    return tuple(F(x) for x in xs)


def dot(a, b):
    return sum((x * y for x, y in zip(a, b)), F(0))


def rref(rows, n):
    """Reduce augmented rows, returning matrix and pivot columns."""
    out = [list(map(F, row)) for row in rows]
    pivots = []
    for col in range(n):
        found = next((i for i in range(len(pivots), len(out))
                      if out[i][col]), None)
        if found is None:
            continue
        pivot = len(pivots)
        out[pivot], out[found] = out[found], out[pivot]
        scale = out[pivot][col]
        out[pivot] = [x / scale for x in out[pivot]]
        for i in range(len(out)):
            if i != pivot and out[i][col]:
                factor = out[i][col]
                out[i] = [a - factor * b for a, b in zip(out[i], out[pivot])]
        pivots.append(col)
    return out, pivots


def solve_unique(equations, n):
    rows, pivots = rref([(*a, b) for a, b in equations], n)
    if any(not any(row[:n]) and row[n] for row in rows):
        return None
    if len(pivots) != n:
        return None
    result = [F(0)] * n
    for i, col in enumerate(pivots):
        result[col] = rows[i][n]
    return tuple(result)


def hull_contains(point, vertices):
    """Find a nonnegative barycentric witness by Caratheodory enumeration."""
    if point in vertices:
        return True
    for size in range(1, min(len(point) + 1, len(vertices)) + 1):
        for chosen in combinations(vertices, size):
            equations = [(tuple(v[j] for v in chosen), point[j])
                         for j in range(len(point))]
            equations.append(((F(1),) * size, F(1)))
            weights = solve_unique(equations, size)
            if weights is not None and all(w >= 0 for w in weights):
                return True
    return False


def canonical(vertices):
    points = tuple(sorted(set(tuple(map(F, p)) for p in vertices)))
    return tuple(p for i, p in enumerate(points)
                 if not hull_contains(p, points[:i] + points[i + 1:]))


def h_vertices(n, equations=(), inequalities=()):
    """Vertices of a bounded rational system; inequalities are a*x <= b.

    An independent oracle for descriptions given as vertices. Equalities may
    be redundant, and the feasible set may be empty or lower-dimensional.
    """
    rows, pivots = rref([(*a, b) for a, b in equations], n)
    if any(not any(row[:n]) and row[n] for row in rows):
        return ()
    dimension = n - len(pivots)
    candidates = set()
    for active in combinations(inequalities, dimension):
        candidate = solve_unique(tuple(equations) + tuple(active), n)
        if candidate is None:
            continue
        if (all(dot(a, candidate) == b for a, b in equations)
                and all(dot(a, candidate) <= b for a, b in inequalities)):
            candidates.add(candidate)
    # All candidates here are vertices in the original H-space.
    return tuple(sorted(candidates))


def nonnegative(n):
    return [(tuple(F(-1 if j == i else 0) for j in range(n)), F(0))
            for i in range(n)]


def simplex_box(bounds):
    n = len(bounds)
    inequalities = []
    for i, (lo, hi) in enumerate(bounds):
        unit = tuple(F(j == i) for j in range(n))
        inequalities.extend([(unit, F(hi)), (tuple(-x for x in unit), -F(lo))])
    return h_vertices(n, [((F(1),) * n, F(1))], inequalities)


def pool(k, l):
    return canonical(k + l)


def mix(k, l, w):
    w = F(w)
    if w == 0:
        return l
    if w == 1:
        return k
    return canonical(tuple(w * a + (1 - w) * b for a, b in zip(p, q))
                     for p, q in product(k, l))


def push(k, mapping, target_size):
    return canonical(tuple(sum((p[i] for i, target in enumerate(mapping)
                                if target == j), F(0))
                           for j in range(target_size)) for p in k)


def condition(k, likelihood):
    return canonical(tuple(x * y / z for x, y in zip(p, likelihood))
                     for p in k if (z := dot(p, likelihood)) > 0)


def bounds(k, payoff):
    assert k, "Conflict is not a valid bound."
    values = [dot(p, payoff) for p in k]
    return min(values), max(values)


def bern(lo, hi=None):
    lo, hi = F(lo), F(lo if hi is None else hi)
    return canonical([(lo, 1 - lo), (hi, 1 - hi)])


@dataclass(frozen=True)
class Model:
    scope: tuple
    vertices: tuple


def model(scope, vertices):
    assert tuple(sorted(set(scope))) == tuple(scope)
    points = canonical(vertices)
    assert all(len(p) == 2 ** len(scope) and sum(p) == 1
               and all(x >= 0 for x in p) for p in points)
    return Model(tuple(scope), points)


def assignments(scope):
    return tuple(product((0, 1), repeat=len(scope)))


def marginal_map(scope, target):
    target_assignments = assignments(target)
    return tuple(target_assignments.index(tuple(x[scope.index(a)] for a in target))
                 for x in assignments(scope))


def project(k, target):
    assert set(target) <= set(k.scope)
    return model(target, push(k.vertices, marginal_map(k.scope, target), 2 ** len(target)))


def combine(k, l):
    """Independent extended H-formulation of the definition.

    Variables are joint masses plus convex weights over both input bases.
    Nonnegative weights of sum one express membership of each marginal.
    Enumerate this extended polytope, then take the exact projected hull.
    """
    scope = tuple(sorted(set(k.scope) | set(l.scope)))
    if not k.vertices or not l.vertices:
        return model(scope, ())
    nj = 2 ** len(scope)
    nk, nl = len(k.vertices), len(l.vertices)
    n = nj + nk + nl
    equations = []
    for operand, start in ((k, nj), (l, nj + nk)):
        row = [F(0)] * n
        row[start:start + len(operand.vertices)] = [F(1)] * len(operand.vertices)
        equations.append((tuple(row), F(1)))
        mapping = marginal_map(scope, operand.scope)
        for outcome in range(2 ** len(operand.scope)):
            row = [F(target == outcome) for target in mapping] + [F(0)] * (n - nj)
            for i, vertex in enumerate(operand.vertices):
                row[start + i] = -vertex[outcome]
            equations.append((tuple(row), F(0)))
    extended = h_vertices(n, equations, nonnegative(n))
    return model(scope, [point[:nj] for point in extended])


def strong_product(k, l):
    return canonical(tuple(a * b for a in p for b in q) for p, q in product(k, l))


passed = []


def checked(name):
    passed.append(name)


# Validate the linear algebra oracle against explicit vertices and degeneracy.
assert solve_unique([(vec(1, 1), F(1)), (vec(1, -1), F(0))], 2) == vec('1/2', '1/2')
assert solve_unique([(vec(1), F(0)), (vec(1), F(1))], 1) is None
assert simplex_box([(0, 1)] * 3) == tuple(sorted([vec(1, 0, 0), vec(0, 1, 0), vec(0, 0, 1)]))
assert simplex_box([(F(1, 2), F(1, 2))] * 2) == (vec('1/2', '1/2'),)
assert simplex_box([(F(3, 4), 1)] * 2) == ()
assert canonical([vec(1, 0), vec(0, 1), vec('1/2', '1/2'), vec(1, 0)]) == bern(0, 1)
checked('oracle: exact solve, inconsistent and lower-dimensional systems, redundant generators')


# P02: nontrivial reassociation weights, empty behavior, exhaustive small terms.
choices = [(), bern(0), bern(1), bern('1/2'), bern('1/4', '3/4'), bern(0, 1)]
p, q = F(2, 5), F(3, 7)
for k, l, m in product(choices, repeat=3):
    assert pool(pool(k, l), m) == pool(k, pool(l, m))
    assert pool(k, l) == pool(l, k)
    assert mix(mix(k, l, p), m, q) == mix(k, mix(l, m, q * (1 - p) / (1 - p * q)), p * q)
    assert mix(pool(k, l), m, p) == pool(mix(k, m, p), mix(l, m, p))
for k in choices:
    assert pool(k, k) == k and mix(k, k, p) == k
    assert mix(k, (), 1) == k and mix((), k, 0) == k
checked('P02/P07: 216 convex-choice triples including conflict and endpoint mixtures')


# P03: compare marginal-coupling formulation against hand-derived Frechet segment.
ka = model(('a',), bern('4/5'))
lb = model(('b',), bern('7/10'))
joint = combine(ka, lb)
expected = canonical([vec('1/2', '3/10', '1/5', 0), vec('7/10', '1/10', 0, '1/5')])
assert joint.vertices == expected
assert combine(lb, ka) == joint
assert combine(joint, joint) == joint
assert combine(joint, ka) == joint
assert project(joint, ('a',)) == ka
assert project(project(joint, ('a',)), ()) == project(joint, ())
assert project(joint, ()).vertices == (vec(1),)
assert combine(joint, model((), [vec(1)])) == joint
implication = model(('a', 'b'), [vec(1, 0, 0, 0), vec(0, 0, 1, 0), vec(0, 0, 0, 1)])
assert combine(joint, implication).vertices == ()

# Unequal overlapping scopes; tests the actual elimination side condition.
wide = model(('a', 'b'), [vec('1/2', 0, 0, '1/2'), vec(0, '1/2', '1/2', 0)])
margin = model(('a',), bern('1/4', '3/4'))
third = model(('b',), bern('1/2'))
assert combine(combine(wide, margin), third) == combine(wide, combine(margin, third))
assert project(combine(margin, wide), ('a',)) == combine(margin, project(wide, ('a',)))
assert combine(wide, project(wide, ('a',))) == wide
checked('P03: extended marginal oracle, Frechet endpoints, scoped laws, zero scope, implication conflict')


# P04/P05: map homomorphisms, payoff pullback, and reduction after mapping.
k3 = canonical([vec(1, 0, 0), vec(0, 1, 0), vec(0, 0, 1)])
l3 = (vec('1/2', '1/4', '1/4'),)
mapping = (0, 0, 1)
assert push(k3, mapping, 2) == bern(0, 1)
triangle = canonical([vec(1, 0, 0), vec(0, 0, 1), vec(0, '1/2', '1/2')])
assert len(triangle) == 3
assert len(set(push((vertex,), mapping, 2)[0] for vertex in triangle)) == 3
assert push(triangle, mapping, 2) == bern(0, 1)
assert push(pool(k3, l3), mapping, 2) == pool(push(k3, mapping, 2), push(l3, mapping, 2))
assert push(mix(k3, l3, p), mapping, 2) == mix(push(k3, mapping, 2), push(l3, mapping, 2), p)
f = vec(3, -2)
assert bounds(push(k3, mapping, 2), f) == bounds(k3, tuple(f[i] for i in mapping))
for k, l in product(choices[1:], repeat=2):
    assert bounds(pool(k, l), f)[0] == min(bounds(k, f)[0], bounds(l, f)[0])
    assert bounds(mix(k, l, p), f)[0] == p * bounds(k, f)[0] + (1 - p) * bounds(l, f)[0]
checked('P04/P05: map reduction, homomorphisms, payoff pullback and choice interpretation')


# X01 and X02: failed lattice and projection rewrites.
fair = model(('a',), bern('1/2'))
heads, tails = model(('a',), bern(1)), model(('a',), bern(0))
assert combine(fair, model(('a',), pool(heads.vertices, tails.vertices))) == fair
assert pool(combine(fair, heads).vertices, combine(fair, tails).vertices) == ()
equal = model(('a', 'b'), [vec('1/2', 0, 0, '1/2')])
unequal = model(('a', 'b'), [vec(0, '1/2', '1/2', 0)])
assert not combine(equal, unequal).vertices
assert combine(project(equal, ('a',)), project(unequal, ('a',))) == fair
checked('X01/X02: nondistributivity and conflict erased by invalid projection pushdown')


# P06/X03: posterior closure, zero-likelihood vertices, and failed rewrites.
assert condition(fair.vertices, vec(1, 0)) == heads.vertices
assert not combine(fair, heads).vertices
assert condition(bern(0, 1), vec(1, 0)) == heads.vertices
assert condition(tails.vertices, vec(1, 0)) == ()
k, l = (vec('3/4', '1/4', 0),), (vec('1/4', 0, '3/4'),)
evidence = vec(0, 1, 1)
posterior = condition(mix(k, l, F(1, 2)), evidence)
assert posterior == (vec(0, '1/4', '3/4'),)
assert mix(condition(k, evidence), condition(l, evidence), F(1, 2)) == (vec(0, '1/2', '1/2'),)
assert posterior != mix(condition(k, evidence), condition(l, evidence), F(1, 2))
lam, eta = vec(1, 2, 0), vec(3, 1, 1)
assert condition(condition(k3, lam), eta) == condition(k3, tuple(a * b for a, b in zip(lam, eta)))
assert condition(pool(k, l), evidence) == pool(condition(k, evidence), condition(l, evidence))
checked('P06/X03: feasible regular extension, impossible event, sequential likelihoods, Bayes reweighting')


# X04/X05: exact box H-oracle, conditioned six-vertex model, every subset event.
c1 = canonical([vec('1/2', '1/2', 0, 0), vec(0, 0, '1/2', '1/2')])
c2 = canonical([vec('1/2', 0, '1/2', 0), vec(0, '1/2', 0, '1/2')])
for i in range(4):
    unit = tuple(F(i == j) for j in range(4))
    assert bounds(c1, unit) == bounds(c2, unit) == (F(0), F(1, 2))
assert bounds(c1, vec(1, 1, 0, 0)) != bounds(c2, vec(1, 1, 0, 0))
prior = simplex_box([(F(1, 10), F(1, 5))] * 3 + [(F(2, 5), F(7, 10))])
posterior4 = condition(prior, vec(1, 1, 1, 0))
posterior3 = canonical([point[:3] for point in posterior4])
expected_posterior = canonical(list(permutations(vec('1/2', '1/4', '1/4')))
                               + list(permutations(vec('2/5', '2/5', '1/5'))))
assert posterior3 == expected_posterior
# Eliminate the evidence mass z from 1/10 <= z*q_i <= 1/5.
# This gives q_i <= 2*q_j for every pair, independently of V-conditioning.
ratio_constraints = [(tuple(F(k == i) - 2 * F(k == j) for k in range(3)), F(0))
                     for i, j in permutations(range(3), 2)]
ratio_hull = h_vertices(3, [(vec(1, 1, 1), F(1))], nonnegative(3) + ratio_constraints)
assert posterior3 == ratio_hull
enclosure = simplex_box([(F(1, 5), F(1, 2))] * 3)
assert enclosure == canonical(permutations(vec('1/2', '3/10', '1/5')))
for event in product((F(0), F(1)), repeat=3):
    assert bounds(posterior3, event) == bounds(enclosure, event)
assert bounds(posterior3, vec(1, -2, 0))[1] == 0
assert bounds(enclosure, vec(1, -2, 0))[1] == F(1, 10)
assert not hull_contains(vec('1/2', '1/5', '3/10'), posterior3)
checked('X04/X05: singleton and all-event information loss, independent H-oracle, separating payoff')


# Decision example: compare payoffs on the same model, not separate ranges.
uncertain = bern('1/5', '4/5')
assert bounds(uncertain, vec(10, 100)) == (F(28), F(82))
assert bounds(uncertain, vec(0, 101)) == (F(101, 5), F(404, 5))
assert bounds(uncertain, vec(10, -1))[0] == F(6, 5)
checked('decision: strict robust preference despite overlapping expected-payoff intervals')


# X06/X11: information combination, convex products, and copying.
fair_b = model(('b',), bern('1/2'))
all_fair_couplings = combine(fair, fair_b)
independent = strong_product(fair.vertices, fair_b.vertices)
assert independent == (vec('1/4', '1/4', '1/4', '1/4'),)
vacuous_product = model(('a', 'b'), strong_product(bern(0, 1), bern(0, 1)))
late_refinement = combine(combine(vacuous_product, fair), fair_b)
assert late_refinement == all_fair_couplings
assert late_refinement.vertices != independent
assert push(fair.vertices, (0, 3), 4) == equal.vertices
assert equal.vertices != independent
checked('X06/X11: late refinement loses factorization; copy, couple, and product are distinct')


# X07: rational moment checks supplement the analytic infinite-extremes proof.
for t, u in product([F(i, 8) for i in range(9)], repeat=2):
    mean, second = (t + u) / 2, (t * t + u * u) / 2
    assert mean * mean <= second <= mean
    assert (second == mean * mean) == (t == u)
checked('X07: 81 exact moment checks; infinite-extreme claim rests on analytic proof')


# X08: witness that some equality pairs occur in PO composition, but points cannot.
def po(a, b):
    intersects = max(a[0], b[0]) <= min(a[1], b[1])
    a_in_b = b[0] <= a[0] and a[1] <= b[1]
    b_in_a = a[0] <= b[0] and b[1] <= a[1]
    return intersects and not a_in_b and not b_in_a


a, b = (F(1, 4), F(3, 4)), (F(0), F(1, 2))
assert po(a, b) and po(b, a)
for lo, hi in combinations([F(i, 8) for i in range(9)], 2):
    assert not po((F(1, 2), F(1, 2)), (lo, hi))
checked('X08: partial equality atom in PO composition; general singleton argument is analytic')


# X09/X10: numerical and recursion falsifiers.
scale = 1 << 63
a, b, c = scale // 4, 5, 7 * scale // 8
rounded_mul = lambda x, y: x * y // scale
assert rounded_mul(rounded_mul(a, b), c) == 0
assert rounded_mul(a, rounded_mul(b, c)) == 1
interval = (F(0), F(1))
for n in range(1, 17):
    new_interval = tuple(F(1, 2) + x / 2 for x in interval)
    assert new_interval == (1 - F(1, 2 ** n), F(1))
    assert new_interval[0] > interval[0] and new_interval[0] < 1
    interval = new_interval
checked('X09/X10: rounded-product nonassociativity and 16 strict refinement steps')


print(json.dumps({
    'status': 'passed',
    'groups': len(passed),
    'checks': passed,
    'limits': 'Tiny exact finite models only; cited and analytic claims are not proved by enumeration. No engine tests.',
}, indent=2))
