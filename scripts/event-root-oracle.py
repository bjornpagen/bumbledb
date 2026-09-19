#!/usr/bin/env python3
"""Generate exact algebraic-root fixtures using independent SymPy 1.14.0.

Only this development generator needs SymPy. Native tests consume the committed
text fixture, not Python or a solver dependency. No floating-point arithmetic is
used to choose a root count, isolating interval, zero test or polynomial sign.
"""
from pathlib import Path
import random

import sympy as s

assert s.__version__ == "1.14.0", s.__version__
x = s.Symbol("x")
probes = [s.Poly(p, x) for p in (x - 1, x*x - 2, 1 + x - x*x, 3 - 2*x + x*x)]
randomizer = random.Random(13743)
coefficients = [
    [0], [1], [-3], [-1, 0, 2], [1, -3, 0, 1], [1, 0, -10, 0, 1],
    [0, 0, 0, 1], [1, 0, 2, 0, 1], [4, 0, -4, 0, 1],
]
for _ in range(96):
    degree = randomizer.randrange(1, 8)
    row = [randomizer.randrange(-5, 6) for _ in range(degree)]
    row.append(randomizer.choice([-5, -3, -1, 1, 3, 5]))
    coefficients.append(row)


def fraction(value):
    numerator, denominator = value.as_numer_denom()
    return f"{numerator}/{denominator}"


def exact_sign(polynomial, interval, probe):
    lower, upper = interval
    if lower == upper:
        return int(s.sign(probe.eval(lower)))
    if s.gcd(polynomial, probe).count_roots(lower, upper):
        return 0
    while probe.count_roots(lower, upper):
        lower, upper = polynomial.refine_root(lower, upper, steps=1)
    return int(s.sign(probe.eval((lower + upper) / 2)))


lines = ["# SymPy 1.14.0 exact rational isolation; seed 13743; ascending coefficients.",
         "# Each root: lower,upper,sign(x-1),sign(x^2-2),sign(1+x-x^2),sign(3-2x+x^2)."]
for row in coefficients:
    polynomial = s.Poly(sum(c*x**i for i, c in enumerate(row)), x)
    encoded = ",".join(str(c) for c in row)
    if polynomial.is_zero:
        lines.append(encoded + "|indeterminate")
        continue
    intervals = polynomial.intervals(eps=s.Rational(1, 100))
    roots = []
    for interval, _multiplicity in intervals:
        # Multiplicity is deliberately absent from the distinct root roster.
        signs = [exact_sign(polynomial, interval, probe) for probe in probes]
        roots.append(",".join([fraction(value) for value in interval] + [str(value) for value in signs]))
    lines.append(encoded + "|" + "|".join(roots))

output = Path(__file__).resolve().parent.parent / "crates/bumbledb-event/tests/fixtures/algebraic-roots-sympy.txt"
output.write_text("\n".join(lines) + "\n")
print(f"Wrote {len(coefficients)} exact polynomial cases to {output}")
