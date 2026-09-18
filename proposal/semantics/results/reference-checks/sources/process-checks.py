#!/usr/bin/env python3
"""Exact reference examples for draft 0.3; not a general inference engine.

Only the standard library is used. Polynomial equality is exact coefficient
equality. Domain, openness, and universal semialgebraic claims rely on the written
proofs in laws.md; finite samples here are explicitly only examples.
"""

from collections import defaultdict
from fractions import Fraction as F
from itertools import product
from math import comb
import json


class Poly:
    """Sparse rational polynomials in two formal variables p and q."""

    def __init__(self, terms=0):
        if isinstance(terms, Poly):
            self.terms = dict(terms.terms)
        elif isinstance(terms, dict):
            self.terms = {k: F(v) for k, v in terms.items() if v}
        else:
            self.terms = {(0, 0): F(terms)} if terms else {}

    def __add__(self, other):
        terms = defaultdict(F, self.terms)
        for k, v in Poly(other).terms.items():
            terms[k] += v
        return Poly(dict(terms))

    __radd__ = __add__

    def __neg__(self):
        return Poly({k: -v for k, v in self.terms.items()})

    def __sub__(self, other):
        return self + -Poly(other)

    def __rsub__(self, other):
        return Poly(other) + -self

    def __mul__(self, other):
        terms = defaultdict(F)
        for (a, b), u in self.terms.items():
            for (c, d), v in Poly(other).terms.items():
                terms[(a + c, b + d)] += u * v
        return Poly(dict(terms))

    __rmul__ = __mul__

    def __pow__(self, n):
        assert isinstance(n, int) and n >= 0
        out = Poly(1)
        for _ in range(n):
            out *= self
        return out

    def __eq__(self, other):
        return self.terms == Poly(other).terms

    def at(self, p=0, q=0):
        return sum((v * F(p)**a * F(q)**b for (a, b), v in self.terms.items()), F(0))

    def substitute(self, p, q):
        return sum((v * p**a * q**b for (a, b), v in self.terms.items()), Poly(0))

    def beta_p(self, a, b):
        return sum(
            (Poly({(0, j): v * beta_moment(a, b, i, 0)})
             for (i, j), v in self.terms.items()), Poly(0))


def rising(a, n):
    out = F(1)
    for i in range(n):
        out *= a + i
    return out


def beta_moment(a, b, h, t):
    a, b = F(a), F(b)
    return rising(a, h) * rising(b, t) / rising(a + b, h + t)


def compose(a, b):
    assert len(a[0]) == len(b)
    return [[sum((a[i][j] * b[j][k] for j in range(len(b))), Poly(0))
             for k in range(len(b[0]))] for i in range(len(a))]


def tensor(a, b):
    return [[a[i][j] * b[u][v]
             for j in range(len(a[0])) for v in range(len(b[0]))]
            for i in range(len(a)) for u in range(len(b))]


p = Poly({(1, 0): 1})
q = Poly({(0, 1): 1})
results = []


def record(name, detail):
    results.append({"name": name, "status": "passed", "detail": detail})


# The three source/outcome interpretations have identical one-draw ranges but
# distinct joint polynomial weights.
shared = [p*p, p*(1-p), (1-p)*p, (1-p)**2]
copied = [p, Poly(0), Poly(0), 1-p]
fresh = [p*q, p*(1-q), (1-p)*q, (1-p)*(1-q)]
assert sum(shared) == sum(copied) == sum(fresh) == 1
assert shared != copied and fresh != shared
assert fresh[1].substitute(p, p) == shared[1]
assert shared[1] == shared[2]
assert (shared[1] + shared[2]) * F(1, 2) == shared[1]
assert copied[1] + copied[2] == 0
assert fresh[1].at(1, 0) == 1 and fresh[2].at(1, 0) == 0
assert fresh[1].at(0, 1) == 0 and fresh[2].at(0, 1) == 1
record("source_vs_outcome_identity", "Exact joint polynomials distinguish shared rate, copied outcome, and fresh rates; disagreement gives fair/shared, impossible/copied, [0,1]/fresh.")


z = 2*p*(1-p)
assert z == F(1, 2) - 2*(p-F(1, 2))**2
assert z.at(F(1, 5)) == z.at(F(4, 5)) == F(8, 25)
assert z.at(F(1, 2)) == F(1, 2)
assert z.at(0) == z.at(1) == 0
assert z != 1
record("fairness_preserves_success_effect", "Extractor posterior is fair, while success is 2p(1-p); exact extremum identity gives [8/25,1/2] on [1/5,4/5].")


prior = [F(1, 10), F(9, 10)]
likelihood = [F(1), F(1, 2)]
weights = [x*y for x, y in zip(prior, likelihood)]
assert weights[0] / sum(weights) == F(2, 11)
assert prior[0] / sum(prior) == F(1, 10)
assert F(2, 11) != F(1, 10)
assert sum(weights) == F(11, 20)
record("early_normalization_falsifier", "Keeping evidence gives posterior 2/11; replacing each row by its normalized output gives 1/10.")


def bernstein(n, k):
    return comb(n, k) * p**k * (1-p)**(n-k)


path_count = 0
for n in range(9):
    by_count = [Poly(0) for _ in range(n+1)]
    for word in product([0, 1], repeat=n):
        weight = Poly(1)
        for bit in word:
            weight *= p if bit else 1-p
        by_count[sum(word)] += weight
        path_count += 1
    for k in range(n+1):
        assert by_count[k] == bernstein(n, k)
        assert bernstein(n, k) == (
            F(n+1-k, n+1) * bernstein(n+1, k)
            + F(k+1, n+1) * bernstein(n+1, k+1))
    assert sum(by_count) == 1
record("bernstein_exchangeability_and_elevation", f"Enumerated {path_count} paths through degree 8; count forms, partition of unity, and each degree-elevation identity agree as exact polynomials.")


a = [[p, 1-p], [1-p, p]]
b = [[q, 1-q], [F(1, 3)*p, F(1, 3)*(1-p)]]
c = [[Poly(1), Poly(0)], [F(1, 2)*(1-p), F(1, 2)*p]]
d = [[Poly(F(1, 3)), Poly(F(2, 3))], [q, 1-q]]
identity = [[Poly(1), Poly(0)], [Poly(0), Poly(1)]]
assert compose(compose(a, b), c) == compose(a, compose(b, c))
assert compose(a, identity) == compose(identity, a) == a
assert compose(tensor(a, b), tensor(c, d)) == tensor(compose(a, c), compose(b, d))
for matrix in [a, b, c, d, compose(a, b), tensor(b, c)]:
    for pv, qv in product([F(0), F(1, 4), F(1, 2), F(1)], repeat=2):
        for row in matrix:
            values = [Poly(x).at(pv, qv) for x in row]
            assert all(x >= 0 for x in values) and sum(values) <= 1
record("kernel_composition_laws", "Symbolic associativity, units, and tensor interchange for shared-parameter filtering kernels; grid validity checks are examples, not the closure proof.")


assert [sum(row) for row in a] == [Poly(1), Poly(1)]
assert [sum(row) for row in b] == [Poly(1), Poly(F(1, 3))]
event = [[Poly(1), Poly(0)], [Poly(0), Poly(0)]]
assert compose(event, event) == event
assert p*p != p
record("discard_and_repeated_evidence", "Total draws discard to one; filtering retains its row likelihood. Reobserving one event is idempotent; two head trials have likelihood p².")


beta_cases = 0
for av, bv in [(1, 1), (2, 3), (F(1, 2), F(3, 2)), (F(2, 3), F(5, 7))]:
    for h, t in product(range(5), repeat=2):
        integrand = p**h * (1-p)**t
        assert integrand.beta_p(av, bv) == beta_moment(av, bv, h, t)
        assert (p*integrand).beta_p(av, bv) == F(av)/(av+bv) * integrand.beta_p(av+1, bv)
        assert ((1-p)*integrand).beta_p(av, bv) == F(bv)/(av+bv) * integrand.beta_p(av, bv+1)
        beta_cases += 1
    for n in range(7):
        assert sum(bernstein(n, k).beta_p(av, bv) for k in range(n+1)) == 1
assert (p*p).beta_p(1, 1) == F(1, 3)
assert p.beta_p(1, 1) * p.beta_p(1, 1) == F(1, 4)
# One head under Beta(1,1), then a further draw from that same source.
assert (p*p).beta_p(1, 1).at() / p.beta_p(1, 1).at() == F(2, 3)
record("beta_binding_and_conjugacy", f"Checked {beta_cases} moment/conjugacy cases, including rational shapes; one shared uniform rate gives HH=1/3 versus 1/4 for fresh allocations; posterior predictive after a head is 2/3.")


assert [x.beta_p(1, 1) for x in shared] == [Poly(F(1, 3)), Poly(F(1, 6)), Poly(F(1, 6)), Poly(F(1, 3))]
assert z.beta_p(1, 1) == F(1, 3)
assert shared[1].beta_p(1, 1) == F(1, 6)
assert shared[1].beta_p(1, 1).at() / z.beta_p(1, 1).at() == F(1, 2)
record("beta_evidence_before_normalization", "Shared uniform-rate extractor integrates to weights (1/6,1/6), success 1/3, and fair posterior; evidence survives the bind.")


# Nonconvex A={0,1}; its convex hull B=[0,1]. Linear one-draw payoff
# extrema agree because an affine function is bounded by its endpoint values.
for hpay, tpay in product(range(-3, 4), repeat=2):
    values = [F(hpay), F(tpay)]
    for pv in [F(0), F(1, 5), F(1, 2), F(4, 5), F(1)]:
        v = pv*hpay + (1-pv)*tpay
        assert min(values) <= v <= max(values)
assert z.at(0) == z.at(1) == 0 and z.at(F(1, 2)) == F(1, 2)
assert (p*p-p).at(0) == (p*p-p).at(1) == 0
assert (p*p-p).at(F(1, 2)) != 0
record("convexification_and_domain_equality", "Endpoint-law union and its hull share one-draw bounds but differ under reuse; p²=p on endpoint laws and fails on the full interval.")


# Derive categorical three-draw path normalization in independent coordinates.
cat = [p, q, 1-p-q]
cat_paths = []
for word in product(range(3), repeat=3):
    weight = Poly(1)
    for value in word:
        weight *= cat[value]
    cat_paths.append(weight)
assert sum(cat_paths) == 1
assert cat[0]*cat[1]*cat[2] == cat[2]*cat[0]*cat[1]
record("categorical_source", "Three-outcome, three-draw path weights normalize symbolically after simplex-coordinate elimination; exchangeable path products agree.")


# Feasible 2x2 joint laws with the required marginals form an affine family.
# t=P(A and B); nonnegativity yields 1/2 <= t <= 7/10.
joint = [p, F(4, 5)-p, F(7, 10)-p, p-F(1, 2)]
assert sum(joint) == 1
assert joint[0]+joint[1] == F(4, 5)
assert joint[0]+joint[2] == F(7, 10)
for endpoint in [F(1, 2), F(7, 10)]:
    assert all(x.at(endpoint) >= 0 for x in joint)
assert F(1, 2) <= F(14, 25) <= F(7, 10)
assert joint[1].at(F(4, 5)) == 0 and joint[2].at(F(4, 5)) < 0
record("assertions_are_all_couplings", "Exact marginal family gives Frechet [1/2,7/10]; A implies B would require t=4/5 and makes another cell negative.")


print(json.dumps({
    "draft": "0.3", "arithmetic": "stdlib Fraction and exact two-variable polynomials",
    "groups_passed": len(results), "checks": results,
    "limits": [
        "No engine implementation, tests, solver benchmarks, or assembly checks.",
        "No general real-algebraic solver or canonical semialgebraic serialization implemented.",
        "Openness, algebraic witnesses, closure, and universal completeness rely on laws.md and cited theorems."
    ]}, indent=2))
