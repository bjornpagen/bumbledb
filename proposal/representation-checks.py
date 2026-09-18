"""Independent bitset oracle versus the proposed canonical BDD/partition carrier.
Reference implementation only: no bumbledb execution or native ownership model.
"""
from fractions import Fraction as Q
from itertools import combinations
from pathlib import Path
import json, runpy

class BDD:
    def __init__(self, variables):
        self.variables = variables
        self.nodes = [None]
        self.unique = {}
        self.cache = {}

    def mk(self, var, lo, hi):
        if lo == hi:
            return lo
        flip = hi & 1
        lo, hi = lo ^ flip, hi ^ flip
        key = (var, lo, hi)
        if key not in self.unique:
            self.unique[key] = len(self.nodes) << 1
            self.nodes.append(key)
        return self.unique[key] ^ flip

    def top(self, ref):
        return self.variables if ref < 2 else self.nodes[ref >> 1][0]

    def cofactor(self, ref, var, bit):
        if self.top(ref) != var:
            return ref
        node = self.nodes[ref >> 1]
        return node[1 + bit] ^ (ref & 1)

    def apply(self, op, a, b):
        if a < 2 and b < 2:
            return (op >> ((a << 1) | b)) & 1
        key = op, a, b
        if key not in self.cache:
            var = min(self.top(a), self.top(b))
            children = [self.apply(op, self.cofactor(a,var,bit), self.cofactor(b,var,bit))
                        for bit in (0,1)]
            self.cache[key] = self.mk(var, *children)
        return self.cache[key]

    def from_bits(self, bits):
        def build(var, prefix):
            if var == self.variables:
                return (bits >> prefix) & 1
            return self.mk(var, build(var+1,prefix), build(var+1,prefix | (1 << var)))
        return build(0,0)

    def evaluate(self, ref, world):
        while ref >= 2:
            var, lo, hi = self.nodes[ref >> 1]
            ref = (hi if (world >> var) & 1 else lo) ^ (ref & 1)
        return ref

    def bits(self, ref):
        return sum(self.evaluate(ref,w) << w for w in range(1 << self.variables))

    def wire(self, support, event):
        # Deterministic reachable postorder, independent of allocation IDs.
        seen, records = {}, []
        def visit(ref):
            if ref < 2:
                return ref
            idx = ref >> 1
            if idx not in seen:
                var, lo, hi = self.nodes[idx]
                low, high = visit(lo), visit(hi)
                seen[idx] = (len(records)+1) << 1
                records.append((var,low,high))
            return seen[idx] ^ (ref & 1)
        s, e = visit(support), visit(event)
        return (self.variables, s, e, tuple(records))

class Space:
    def __init__(self, bdd, support_bits):
        assert support_bits
        self.bdd = bdd
        self.support = bdd.from_bits(support_bits)
        self.pairs = [(0,self.support)]
        self.unique = {self.pairs[0]:0}

    def intern(self, raw):
        t = self.bdd.apply(0x8,self.support,raw)
        f = self.bdd.apply(0x8,self.support,raw ^ 1)
        assert t != f
        pair = tuple(sorted((t,f)))
        if pair not in self.unique:
            self.unique[pair] = len(self.pairs)
            self.pairs.append(pair)
        return (self.unique[pair] << 1) | int(t == pair[1])

    def root(self, event):
        return self.pairs[event >> 1][event & 1]

    def apply(self, op, a, b):
        return self.intern(self.bdd.apply(op,self.root(a),self.root(b)))

    def bits(self, event):
        return self.bdd.bits(self.root(event))

    def signature(self, a, b):
        return sum(int(self.apply(1 << cell,a,b) != 0) << cell for cell in range(4))

def oracle(op,a,b,support,width):
    return sum((((op >> ((((a>>w)&1)<<1) | ((b>>w)&1))) & 1)
                & ((support>>w)&1)) << w for w in range(width))

def run():
    pairs_checked = operations = 0
    for support in range(1,16):
        bdd = BDD(2)
        space = Space(bdd,support)
        by_bits = {}
        for raw in range(16):
            e = space.intern(bdd.from_bits(raw))
            bits = raw & support
            assert by_bits.setdefault(bits,e) == e
            assert space.bits(e) == bits
            assert space.bits(e ^ 1) == (support ^ bits)
        assert by_bits[0] == 0 and by_bits[support] == 1
        for a,ea in by_bits.items():
            for b,eb in by_bits.items():
                pairs_checked += 1
                sig = space.signature(ea,eb)
                expected_sig = 0
                for w in range(4):
                    if (support >> w) & 1:
                        expected_sig |= 1 << ((((a>>w)&1)<<1) | ((b>>w)&1))
                assert sig == expected_sig and sig != 0
                assert space.signature(ea ^ 1,eb) == (((sig << 2) | (sig >> 2)) & 15)
                assert space.signature(ea,eb ^ 1) == (((sig & 5) << 1) | ((sig & 10) >> 1))
                assert space.signature(eb,ea) == ((sig & 9) | ((sig & 2)<<1) | ((sig & 4)>>1))
                for op in range(16):
                    result = space.apply(op,ea,eb)
                    assert space.bits(result) == oracle(op,a,b,support,4)
                    assert space.apply(op ^ 15,ea,eb) == (result ^ 1)
                    operations += 1
        # Independent construction order must yield identical logical bytes.
        other = BDD(2)
        for bits in reversed(range(16)):
            other.from_bits(bits)
        other_space = Space(other,support)
        for bits,e in by_bits.items():
            f = other_space.intern(other.from_bits(bits))
            assert bdd.wire(space.support,space.root(e)) == other.wire(other_space.support,other_space.root(f))
        for var,lo,hi in bdd.nodes[1:]:
            assert lo != hi and not (hi & 1)
            assert bdd.top(lo) > var and bdd.top(hi) > var

    # The guard bits p=0 and p=1 are logical, not random. Bit 2 is heads.
    # Explicit Supported(law) fixture, not the default structural universe.
    # Feasible positive-support cells: interior/tails, interior/heads,
    # endpoint-zero/tails, endpoint-one/heads.
    guarded = BDD(3)
    guarded_space = Space(guarded,sum(1 << w for w in (0,4,1,6)))
    heads = guarded_space.intern(guarded.from_bits(0xf0))
    assert guarded_space.bits(heads) == (1 << 4) | (1 << 6)
    for p in (Q(0),Q(1,5),Q(1,2),Q(1)):
        guard = 1 if p == 0 else 2 if p == 1 else 0
        masses = {guard:1-p, guard | 4:p}
        assert sum(masses.values()) == 1
        measured = sum(m for w,m in masses.items() if guarded.evaluate(guarded_space.root(heads),w))
        assert measured == p
    endpoint = Space(BDD(3),1 << 1)
    assert endpoint.intern(endpoint.bdd.from_bits(0xf0)) == 0

    # Coup as an explicitly finite scenario; world-index ordering is a semantic
    # test fixture, not the proposed generative coordinate order or a benchmark.
    base = runpy.run_path(str(Path(__file__).with_name('coup')/'event-checks.py'))
    cards, assignment, holds = (base[k] for k in ('CARDS','assignment','holds'))
    alice = ('Duke1','Assassin1')
    rest = tuple(c for c in cards if c not in alice)
    deals = [assignment({'A':alice,'B':b,'C':c}) for b in combinations(rest,2)
             for c in combinations(tuple(x for x in rest if x not in b),2)]
    bdd = BDD(13)
    space = Space(bdd,(1 << len(deals))-1)
    def held(seat,role):
        bits = sum(1 << w for w in holds(deals,seat,role))
        return space.intern(bdd.from_bits(bits))
    b,c,a = held('B','Captain'),held('C','Captain'),held('C','Ambassador')
    e = space.apply(0x8,b,space.apply(0xe,c,a) ^ 1)
    fraction = Q(bin(space.bits(e)).count('1'),len(deals))
    assert fraction == Q(189,1430)
    return {'scope':'Representation reference only, not the native engine.',
            'groups_passed':['support_relative_canonical_identity','constant_time_relative_complement',
                             'all_sixteen_boolean_operations','venn_signature_and_symmetries',
                             'deterministic_wire_under_reallocation','reduced_ordered_complement_normalization',
                             'logical_guards_and_endpoint_support','coup_composite_through_bdd_pairs'],
            'support_masks':15,'event_pairs':pairs_checked,'boolean_operations':operations,
            'coup_worlds':len(deals),'coup_steal_support':str(fraction),
            'coup_fixture_bdd_nodes':len(bdd.nodes)-1,'coup_fixture_partition_pairs':len(space.pairs)}

if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
