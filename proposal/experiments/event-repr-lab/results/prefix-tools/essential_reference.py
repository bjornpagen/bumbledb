"""Finite reference for the essential-coordinate table normal form.

Correctness evidence only. This is not a Rust carrier or a benchmark. Actual
world enumeration is restricted to at most four coordinates in this checker.
"""
from pathlib import Path
import functools, hashlib, json, random


def cofactor_word(word, width, axis, high):
    out = 0
    low_mask = (1 << axis) - 1
    for i in range(1 << (width - 1)):
        old = (i & low_mask) | (high << axis) | ((i >> axis) << (axis + 1))
        out |= ((word >> old) & 1) << i
    return out


class Arena:
    def __init__(self, width, order, limit):
        self.width, self.order, self.limit = width, tuple(order), limit
        self.rank = {v: i for i, v in enumerate(order)}
        self.nodes = [('constant',)]
        self.axes = [frozenset()]
        self.unique = {}
        # Instance caches are released with the reference arena.
        self.table = functools.lru_cache(None)(self.table)
        self.cofactor = functools.lru_cache(None)(self.cofactor)
        self.apply = functools.lru_cache(None)(self.apply)

    def intern(self, key, axes):
        if key in self.unique: return self.unique[key]
        ref = len(self.nodes) * 2
        self.unique[key] = ref
        self.nodes.append(key)
        self.axes.append(frozenset(axes))
        return ref

    def variables(self, ref): return self.axes[ref >> 1]

    def evaluate(self, ref, world):
        key = self.nodes[ref >> 1]
        if key[0] == 'constant': value = 0
        elif key[0] == 'table':
            _, axes, word = key
            index = sum(((world >> v) & 1) << i for i, v in enumerate(axes))
            value = (word >> index) & 1
        else:
            _, v, low, high = key
            value = self.evaluate(high if (world >> v) & 1 else low, world)
        return value ^ (ref & 1)

    def table(self, axes, word):
        assert len(set(axes)) == len(axes) and all(0 <= v < self.width for v in axes)
        assert 0 <= word < (1 << (1 << len(axes)))
        ordered = tuple(sorted(axes))
        if axes != ordered:
            reordered = 0
            for i in range(1 << len(axes)):
                old = sum(((i >> ordered.index(v)) & 1) << k for k, v in enumerate(axes))
                reordered |= ((word >> old) & 1) << i
            axes, word = ordered, reordered
        index = 0
        while index < len(axes):
            low = cofactor_word(word, len(axes), index, 0)
            high = cofactor_word(word, len(axes), index, 1)
            if low == high:
                word = low
                axes = axes[:index] + axes[index + 1:]
            else: index += 1
        if not axes: return word
        if len(axes) <= self.limit:
            flip = (word >> ((1 << len(axes)) - 1)) & 1
            if flip: word ^= (1 << (1 << len(axes))) - 1
            return self.intern(('table', axes, word), axes) ^ flip
        v = min(axes, key=self.rank.__getitem__)
        pos = axes.index(v)
        rest = axes[:pos] + axes[pos + 1:]
        low = self.table(rest, cofactor_word(word, len(axes), pos, 0))
        high = self.table(rest, cofactor_word(word, len(axes), pos, 1))
        return self.branch(v, low, high)

    def branch(self, v, low, high):
        assert 0 <= v < self.width
        assert all(self.rank[w] > self.rank[v] for w in self.variables(low) | self.variables(high))
        if low == high: return low
        axes = tuple(sorted({v} | self.variables(low) | self.variables(high)))
        if len(axes) <= self.limit:
            word = 0
            for i in range(1 << len(axes)):
                world = sum(((i >> k) & 1) << a for k, a in enumerate(axes))
                word |= self.evaluate(high if world >> v & 1 else low, world) << i
            return self.table(axes, word)
        flip = high & 1
        return self.intern(('branch', v, low ^ flip, high ^ flip), axes) ^ flip

    def cofactor(self, ref, v, high):
        if v not in self.variables(ref): return ref
        key = self.nodes[ref >> 1]
        if key[0] == 'table':
            _, axes, word = key
            pos = axes.index(v)
            out = self.table(axes[:pos] + axes[pos + 1:], cofactor_word(word, len(axes), pos, high))
        else:
            _, top, low, upper = key
            out = (upper if high else low) if top == v else self.branch(
                top, self.cofactor(low, v, high), self.cofactor(upper, v, high))
        return out ^ (ref & 1)

    def apply(self, op, a, b):
        axes = tuple(sorted(self.variables(a) | self.variables(b)))
        if len(axes) <= self.limit:
            word = 0
            for i in range(1 << len(axes)):
                world = sum(((i >> k) & 1) << v for k, v in enumerate(axes))
                cell = 2 * self.evaluate(a, world) + self.evaluate(b, world)
                word |= ((op >> cell) & 1) << i
            return self.table(axes, word)
        v = min(axes, key=self.rank.__getitem__)
        return self.branch(v,
            self.apply(op, self.cofactor(a, v, 0), self.cofactor(b, v, 0)),
            self.apply(op, self.cofactor(a, v, 1), self.cofactor(b, v, 1)))

    def export(self, ref):
        return sum(self.evaluate(ref, w) << w for w in range(1 << self.width))

    def exists(self, ref, coordinates):
        for v in coordinates:
            ref = self.apply(14, self.cofactor(ref, v, 0), self.cofactor(ref, v, 1))
        return ref


def main():
    cases = reconstructions = booleans = quantifiers = 0
    rng = random.Random(20260917)
    for width in [2, 3, 4]:
        full = (1 << (1 << width)) - 1
        axes = tuple(range(width))
        for order in [axes, axes[::-1]]:
            for limit in range(1, width + 1):
                c = Arena(width, order, limit)
                literals = [c.table((v,), 2) for v in axes]
                for word in range(full + 1):
                    ref = c.table(axes, word)
                    assert c.export(ref) == word
                    assert c.table(axes, full ^ word) == ref ^ 1
                    cases += 1
                    if width < 4 or word % 251 == 0:
                        for v in axes:
                            low, high = c.cofactor(ref, v, 0), c.cofactor(ref, v, 1)
                            combined = c.apply(14, c.apply(8, literals[v], high), c.apply(4, low, literals[v]))
                            assert combined == ref
                            reconstructions += 1
                        other_word = rng.randrange(full + 1)
                        other = c.table(axes, other_word)
                        for op in range(16):
                            expected = sum(((op >> (2 * ((word >> w) & 1) + ((other_word >> w) & 1))) & 1) << w for w in range(1 << width))
                            assert c.apply(op, ref, other) == c.table(axes, expected)
                            booleans += 1
                        for mask in range(1 << width):
                            expected = sum(any(((word >> v) & 1) and (v & ~mask) == (w & ~mask) for v in range(1 << width)) << w for w in range(1 << width))
                            assert c.exists(ref, [v for v in axes if mask >> v & 1]) == c.table(axes, expected)
                            quantifiers += 1
                print(f'Checked width={width}, order={order}, table_limit={limit}', flush=True)
    # Arbitrarily separated names still occupy a single bounded local table.
    c = Arena(60, range(59, -1, -1), 2)
    event = c.table((59, 0), 6)
    assert c.variables(event) == {0, 59}
    assert c.nodes[event >> 1][0] == 'table'
    assert c.exists(event, [59]) == 1
    result = {'passed': True, 'function_cases': cases, 'shannon_reconstructions': reconstructions,
              'boolean_cases': booleans, 'quantifier_cases': quantifiers,
              'scope': 'Finite independent reference, including all functions up to four coordinates, two physical orders and every table limit 1..width; sampled arbitrary Shannon reconstruction, all Boolean operators and all quantifier masks. One sixty-coordinate noncontiguous local-table case. Not Rust or performance evidence.',
              'checker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (Path(__file__).parent/'results/essential-reference.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))

if __name__ == '__main__': main()
