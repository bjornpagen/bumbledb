"""Exhaust information partitions independently of coordinate/decoder encodings."""
from pathlib import Path
import hashlib
import json

LAB = Path(__file__).resolve().parent


def partitions(n):
    if n == 0:
        return [()]
    result = []
    for cells in partitions(n-1):
        bit = 1 << (n-1)
        for i in range(len(cells)):
            result.append(cells[:i]+(cells[i] | bit,)+cells[i+1:])
        result.append(cells+(bit,))
    return result


def tables(n, cells):
    events = range(1 << n)
    possible = [sum(cell for cell in cells if cell & a) for a in events]
    guaranteed = [sum(cell for cell in cells if cell & a == cell) for a in events]
    return possible, guaranteed


counts = dict(partitions=0, events=0, binary_laws=0, filter_iff=0,
              observable_bounds=0, observation_order_iff=0, nested_readouts=0)
for n, bell in enumerate([1, 1, 2, 5, 15]):
    full = (1 << n)-1
    events = range(1 << n)
    parts = partitions(n)
    assert len(parts) == bell and len(set(parts)) == bell
    compiled = [(cells, *tables(n, cells)) for cells in parts]
    for cells, p, g in compiled:
        counts['partitions'] += 1
        stable = [b for b in events if p[b] == b]
        for a in events:
            assert g[a] & ~a == 0 and a & ~p[a] == 0
            assert g[a] == full ^ p[full ^ a]
            assert p[p[a]] == p[a] and g[g[a]] == g[a]
            assert p[g[a]] == g[a] and g[p[a]] == p[a]
            ambiguity = p[a] & ~g[a]
            assert p[ambiguity] == g[ambiguity] == ambiguity
            assert (p[a] == a) == (g[a] == a)
            # Forced true, forced false and ambiguity partition legal worlds.
            assert g[a] | g[full ^ a] | ambiguity == full
            assert not (g[a] & g[full ^ a] or g[a] & ambiguity or g[full ^ a] & ambiguity)
            counts['events'] += 1
            for b in events:
                assert p[a | b] == p[a] | p[b]
                assert g[a & b] == g[a] & g[b]
                counts['binary_laws'] += 1
            for b in stable:
                assert (p[a] & ~b == 0) == (a & ~b == 0)
                assert (b & ~g[a] == 0) == (b & ~a == 0)
                counts['observable_bounds'] += 1
        for b in events:
            membership_fd = all((a & b in [0, a]) for a in cells)
            assert (p[b] == b) == membership_fd
            assert all(p[a & b] == p[a] & b for a in events) == membership_fd
            assert all(g[a | b] == g[a] | b for a in events) == membership_fd
            counts['filter_iff'] += 1
    for cells_f, pf, gf in compiled:
        for cells_g, pg, gg in compiled:
            refines = all(any(cf & ~cg == 0 for cg in cells_g) for cf in cells_f)
            assert all(pf[a] & ~pg[a] == 0 for a in events) == refines
            assert all(gg[a] & ~gf[a] == 0 for a in events) == refines
            counts['observation_order_iff'] += 1
            if refines:
                for a in events:
                    assert pg[pf[a]] == pf[pg[a]] == pg[a]
                    assert gg[gf[a]] == gf[gg[a]] == gg[a]
                    counts['nested_readouts'] += 1

p, g = tables(2, (3,))
assert p[1] & p[2] == 3 and p[1 & 2] == 0
assert g[1 | 2] == 3 and g[1] | g[2] == 0
result = dict(passed=True, worlds=list(range(5)), cases=counts,
              counterexamples={'separate_possible_loses_joint_witness': True,
                               'separate_guaranteed_loses_joint_coverage': True},
              boundary='All partitions of zero through four admitted worlds. Independent set oracle; no coordinate encodings, raw aliases or probabilities.',
              checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
(LAB/'results/readout-laws-reference.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
