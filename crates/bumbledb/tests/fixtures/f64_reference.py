"""Independent exact-rational golden generator; stdout only, no dependencies.

Every finite input is decoded to fractions.Fraction using integer shifts.
Output rounding uses binary search over canonical positive binary64 bit words,
then exact rational distance comparisons/tie parity. It never calls the Rust
implementation, native float arithmetic, float(), or decimal parsing.

Regenerate with python3 f64_reference.py and compare stdout to f64_reference.txt.
"""

from fractions import Fraction
from functools import lru_cache
from random import Random

SIGN = 1 << 63
INF = 0x7FF0000000000000
NAN = 0x7FF8000000000000
MAX = INF - 1


@lru_cache(maxsize=32768)
def finite(bits):
    exponent, fraction = (bits >> 52) & 2047, bits & ((1 << 52) - 1)
    significand = fraction if exponent == 0 else (1 << 52) + fraction
    shift = -1074 if exponent == 0 else exponent - 1075
    result = Fraction(significand << shift) if shift >= 0 else Fraction(significand, 1 << -shift)
    return -result if bits & SIGN else result


def canonical(bits):
    if bits & INF == INF and bits & ((1 << 52) - 1):
        return NAN
    return 0 if bits & (SIGN - 1) == 0 else bits


def rounded(value):
    if value == 0:
        return 0
    sign = SIGN if value < 0 else 0
    value = abs(value)
    # Overflow is decided at the exact midpoint to the hypothetical 2^1024.
    if value >= Fraction((1 << 1024) - (1 << 970)):
        return sign | INF
    lo, hi = 0, MAX
    while lo < hi:
        middle = (lo + hi + 1) // 2
        if finite(middle) <= value:
            lo = middle
        else:
            hi = middle - 1
    if lo == MAX:
        return sign | MAX
    below, above = value - finite(lo), finite(lo + 1) - value
    choice = lo + (above < below or (above == below and lo & 1))
    return 0 if choice == 0 else sign | choice


def operation(op, a, b):
    if a == NAN or b == NAN:
        return NAN
    if op == "sub":
        b = canonical(b ^ SIGN)
        op = "add"
    ai, bi = a & (SIGN - 1) == INF, b & (SIGN - 1) == INF
    sign = (a ^ b) & SIGN
    if op == "add":
        if ai and bi:
            return NAN if sign else a
        if ai or bi:
            return a if ai else b
        return rounded(finite(a) + finite(b))
    if op == "mul":
        if ai or bi:
            return NAN if a == 0 or b == 0 else sign | INF
        return rounded(finite(a) * finite(b))
    if ai and bi or a == 0 and b == 0:
        return NAN
    if ai or b == 0:
        return sign | INF
    if bi:
        return 0
    return rounded(finite(a) / finite(b))


def reduction(values):
    if NAN in values or INF in values and SIGN | INF in values:
        return NAN, NAN
    if INF in values:
        return INF, INF
    if SIGN | INF in values:
        return SIGN | INF, SIGN | INF
    total = sum((finite(v) for v in values), Fraction())
    return rounded(total), rounded(total / len(values))


def main():
    print("# Exact Fraction/binary-search reference; seed 0xbdbf64; all words hexadecimal.")
    random = Random(0xBDBF64)
    edges = [0, 1, 2, (1 << 52) - 1, 1 << 52, (1 << 52) + 1,
             0x3CA0000000000000, 0x3FEFFFFFFFFFFFFF, 0x3FF0000000000000,
             0x3FF0000000000001, 0x4000000000000000, 0x4340000000000000,
             0x4340000000000001, MAX - 1, MAX, INF, NAN]
    edges += [canonical(v | SIGN) for v in edges if v not in (0, NAN)]
    pairs = [(a, b) for a in edges for b in edges]
    pairs += [(canonical(random.getrandbits(64)), canonical(random.getrandbits(64))) for _ in range(512)]
    for op in ("add", "sub", "mul", "div"):
        for a, b in pairs:
            print(f"{op} {a:016x} {b:016x} {operation(op, a, b):016x}")
    groups = [[0], [1, 1], [1, 0], [1, 1, 0], [MAX, MAX], [SIGN | MAX, SIGN | MAX],
              [0x4341C37937E08000, 0x3FF0000000000000, 0xC341C37937E08000],
              [NAN, INF, SIGN | INF], [INF, SIGN | INF], [INF, MAX], [SIGN | INF, 0]]
    groups += [[canonical(random.getrandbits(64)) for _ in range(random.randrange(1, 24))] for _ in range(256)]
    groups += [[a, canonical(a ^ SIGN), b] for a, b in pairs[::31]]
    for values in groups:
        total, mean = reduction(values)
        print(f"reduce {total:016x} {mean:016x} " + " ".join(f"{v:016x}" for v in values))
    for value in [0, 1, (1 << 53) - 1, 1 << 53, (1 << 53) + 1, (1 << 53) + 3,
                  (1 << 63) - 1, 1 << 63, (1 << 64) - 1] + [random.getrandbits(64) for _ in range(256)]:
        bits = rounded(Fraction(value))
        exact = int(finite(bits) == value)
        print(f"u64 {value} {bits:016x} {exact}")
    for value in [-(1 << 63), -(1 << 53) - 1, -1, 0, 1, (1 << 53) + 1, (1 << 63) - 1] + [random.getrandbits(64) - (1 << 63) for _ in range(256)]:
        bits = rounded(Fraction(value))
        exact = int(finite(bits) == value)
        print(f"i64 {value} {bits:016x} {exact}")


if __name__ == "__main__":
    main()
