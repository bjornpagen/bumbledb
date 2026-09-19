#!/usr/bin/env python3
"""Generate independent exact minimal-polynomial/root-ordinal fixtures.

SymPy 1.14.0 supplies rational factorization and real isolation. Native tests
consume committed text; the library has no Python or external solver dependency.
"""
from pathlib import Path
import random
import sympy as s

assert s.__version__ == "1.14.0", s.__version__
x = s.Symbol("x")
randomizer = random.Random(293781)
polynomials = [
    (x*x-2)*(x*x-3), (x*x-2)**2*(x+3),
    (2*x-1)**3*(x*x-2), (x**3-2)*(x*x-2),
    x**4-x-1, x**5-x-1, x**6-x-1,
    -s.Rational(7, 3)*(x*x-2)*(x*x+1),
]
for _ in range(96):
    degree = randomizer.randrange(1, 6)
    coefficients = [randomizer.randrange(-4, 5) for _ in range(degree)]
    coefficients.append(randomizer.choice([-3, -1, 1, 3]))
    polynomials.append(sum(c*x**i for i, c in enumerate(coefficients)))

def fraction(value):
    n, d = value.as_numer_denom()
    return f"{n}/{d}"

def coefficients(poly):
    return ",".join(fraction(c) for c in reversed(poly.all_coeffs()))

lines = ["# SymPy 1.14.0; exact QQ factorization/isolation; seed 293781.",
         "# input ascending rational coefficients|lower,upper|minimal monic coefficients|real-root ordinal"]
for expression in polynomials:
    polynomial = s.Poly(expression, x, domain=s.QQ)
    factors = [factor.monic() for factor, _ in polynomial.factor_list()[1]]
    for (lower, upper), _ in polynomial.intervals(eps=s.Rational(1, 100)):
        matched = [f for f in factors if (f.eval(lower) == 0 if lower == upper else f.count_roots(lower, upper) == 1)]
        assert len(matched) == 1
        minimal = matched[0]
        ordinal = minimal.count_roots(s.S.NegativeInfinity, lower) - int(lower == upper)
        lines.append("|".join([coefficients(polynomial), f"{fraction(lower)},{fraction(upper)}", coefficients(minimal), str(ordinal)]))
output = Path(__file__).resolve().parent.parent / "crates/bumbledb-event/tests/fixtures/algebraic-identity-sympy.txt"
output.write_text("\n".join(lines) + "\n")
print(f"Wrote {len(lines)-2} exact numeric identities from {len(polynomials)} polynomials to {output}")
