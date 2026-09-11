#!/usr/bin/env python3
"""Independent structural oracle. Check by default; --write regenerates its fixture."""

import argparse
from fractions import Fraction
import json
from pathlib import Path
import random


def quotient(kind, a, b, divisor, mode):
    if divisor == 0:
        return "divisionByZero"
    if divisor < 0:
        return "nonPositiveDivisor"
    exact = Fraction(a * b, divisor)
    if mode == "towardZero":
        result = int(exact)
    else:
        lower = exact.numerator // exact.denominator
        # Choose among neighboring integers by rational distance, then the
        # requested tie rule. This is separate from native remainder rounding.
        tie = (lambda n: -abs(n)) if mode == "nearestTiesAwayFromZero" else (lambda n: n % 2)
        result = min((lower, lower + 1), key=lambda n: (abs(exact - n), tie(n)))
    low = -(1 << 63) if kind == "i64" else 0
    high = (1 << 63) - 1 if kind == "i64" else (1 << 64) - 1
    return str(result) if low <= result <= high else "overflow"


def corpus():
    rng = random.Random(121)
    quotients = []
    for kind in ["i64", "u64"]:
        low = -(1 << 63) if kind == "i64" else 0
        high = (1 << 63) - 1 if kind == "i64" else (1 << 64) - 1
        values = sorted(set([low, high, 0, 1, 2, 3, 5, 7] + ([-1, -2, -3, -5, -7] if kind == "i64" else [])))
        divisors = [0, 1, 2, 3, 4, high] + ([-1] if kind == "i64" else [])
        samples = [(a, 1, d) for a in values for d in divisors]
        samples += ([(high, 2, 2), (low, -1, 2), (low, -1, 1)] if kind == "i64"
                    else [(high, 2, 2), (high, high, high), (high, 2, 1)])
        samples += [(rng.randint(low, high), rng.randint(low, high), rng.randint(1, high)) for _ in range(1024)]
        # Both sides of ties and the result-range boundary, including wide
        # products that become representable only after division.
        for d in [2, 4, 10, 1 << 32, high - 1]:
            for sign in ([1, -1] if kind == "i64" else [1]):
                samples += [(sign * (d // 2 + offset), 1, d) for offset in [-1, 0, 1]]
            samples += [(high, d, d), (high - 1, d, d), (low, d, d)]
        for a, b, divisor in samples:
            for mode in ["towardZero", "nearestTiesAwayFromZero", "nearestTiesToEven"]:
                quotients.append([kind, str(a), str(b), str(divisor), mode, quotient(kind, a, b, divisor, mode)])

    spans = [(a, b) for a in range(-2, 3) for b in range(a + 1, 4)] + [
        (-(1 << 63), (1 << 63) - 1), (-1, (1 << 63) - 1),
        (-(1 << 63), -(1 << 63) + 1), (-(1 << 63), (1 << 63) - 2),
        ((1 << 63) - 2, (1 << 63) - 1), (-1, 1), (0, 1 << 32)]
    intervals = []
    for a, b in spans:
        for c, d in spans:
            bounds = sorted(set([a, b, c, d]))
            selected = {"common": [], "rest": []}
            # Classify endpoint cells by membership, then coalesce neighbors.
            for x, y in zip(bounds, bounds[1:]):
                if a <= x and y <= b:
                    lane = "common" if c <= x and y <= d else "rest"
                    if selected[lane] and selected[lane][-1][1] == x:
                        selected[lane][-1][1] = y
                    else:
                        selected[lane].append([x, y])
            def interval(span):
                return [str(n) for n in span]
            intervals.append([interval((a, b)), interval((c, d)),
                              [interval(s) for s in selected["common"]],
                              [interval(s) for s in selected["rest"]]])
    return quotients, intervals


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    quotients, intervals = corpus()
    compact = lambda row: json.dumps(row, separators=(",", ":"))
    rendered = ('{\n"quotients":[\n' + ',\n'.join(map(compact, quotients))
                + '\n],\n"intervals":[\n' + ',\n'.join(map(compact, intervals)) + '\n]}\n')
    path = Path(__file__).resolve().parent.parent / "crates/bumbledb-bench/fixtures/conformance/structural-algebra.json"
    if args.write:
        path.write_text(rendered)
    elif path.read_text() != rendered:
        raise SystemExit("structural corpus differs from independent oracle")
    print(f"structural corpus: {len(quotients)} quotient cases, {len(intervals)} interval pairs")


if __name__ == "__main__":
    main()
