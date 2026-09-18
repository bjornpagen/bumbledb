"""Exhaustive row-factorization oracle; not a native timing."""
from pathlib import Path
import itertools,json,hashlib
LAB=Path(__file__).resolve().parent.parent
ROOT=LAB.parents[2]
checked=0;factorable=0
for relation in range(256):
    tuples=[t for t in itertools.product(range(2),repeat=3) if relation>>(t[0]*4+t[1]*2+t[2])&1]
    marginals=[set(t[i] for t in tuples) for i in range(3)]
    rectangle=all(t in tuples for t in itertools.product(*marginals))
    factors=True
    for masks in itertools.product(range(4),repeat=3):
        joined=any(all(masks[i]>>t[i]&1 for i in range(3)) for t in tuples)
        product=all(any(masks[i]>>v&1 for v in marginals[i]) for i in range(3))
        factors &= joined==product;checked+=1
    assert factors==rectangle
    factorable+=factors
assert (checked,factorable)==(16384,28)
files=['proposal/research/event-algebra/abo-khamis-ngo-rudra-2016-faq.txt','proposal/papers/abo-khamis-ngo-rudra-2016-faq.pdf']
out=dict(passed=True,relations=256,local_predicate_checks=checked,factorable_relations=factorable,
    source_hashes={n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in files},
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    reading_scope='FAQ section 1.2 definition/equation (1) and section 2.2 variable elimination/distributivity; native Free Join complete-binding callback and clover mappings.',
    boundary='Finite semantic oracle for universal row-factorization condition. No timing or native implementation claim.')
(LAB/'results/factorized-pack-oracle.json').write_text(json.dumps(out,indent=2)+'\n')
print('Factorization oracle passed:',checked,'predicate checks;',factorable,'rectangular relations')
