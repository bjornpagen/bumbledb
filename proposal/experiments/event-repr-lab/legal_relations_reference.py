"""Independent finite role/domain checks. Run outside native timing sweeps."""
from pathlib import Path
from itertools import product
import hashlib,json,random

LAB=Path(__file__).resolve().parent
rng=random.Random(20260918)
counts=dict(domain_sets=0,relation_pairs=0,associativity=0,residual_adjunction=0,role_checks=0,coupled_readout_checks=0)

def compose(r,q):
    return {(x,z) for x,y in r for yy,z in q if y==yy}

def residual(d,r,t):
    return {(y,z) for y in d for z in d if all((x,y) not in r or (x,z) in t for x in d)}

def lift(d,r):
    return {(x,y,z) for x,y in r for z in d}

def stages(s,a,b):
    # Source -> destination coordinate movement, clipping after each step.
    pb={(z,x,y) for x,y,z in b}&s
    joined=a&pb
    hidden={(x,k,z) for x,y,z in joined for k in range(4)}&s
    return {(x,z,y) for x,y,z in hidden}&s

def subsets(points):
    points=list(points)
    return [{p for i,p in enumerate(points) if code>>i&1} for code in range(1<<len(points))]

for dm in range(16):
    d={v for v in range(4) if dm>>v&1}
    s=set(product(d,repeat=3));points=list(product(sorted(d),repeat=2))
    counts['domain_sets']+=1
    if len(d)<=2:
        rels=subsets(points);pairs=list(product(rels,repeat=2));triples=product(rels,repeat=3)
        events=subsets(sorted(s))
    else:
        rels=[set(),set(points),{(v,v) for v in d}]
        rels += [{p for p in points if rng.getrandbits(1)} for _ in range(125)]
        pairs=[(r,rels[(i*17+11)%len(rels)]) for i,r in enumerate(rels)]
        triples=[(r,rels[(i*7+3)%len(rels)],rels[(i*13+2)%len(rels)]) for i,r in enumerate(rels)]
        events=[lift(d,r) for r in rels]+[{p for p in s if rng.getrandbits(1)} for _ in range(128)]
    identity={(x,x) for x in d}
    for r in rels:
        assert compose(identity,r)==r==compose(r,identity)
    for r,q in pairs:
        counts['relation_pairs']+=1
        assert stages(s,lift(d,r),lift(d,q))==lift(d,compose(r,q))
    for r,q,t in triples:
        counts['associativity']+=1
        assert compose(compose(r,q),t)==compose(r,compose(q,t))
        counts['residual_adjunction']+=1
        assert (compose(r,q)<=t)==(q<=residual(d,r,t))
    for f in events:
        counts['role_checks']+=1
        projected={(x,y,z) for x,y,t in f for z in d}
        cylinder=all(((x,y,z) in f)==((x,y,t) in f)
                     for x,y,z,t in product(d,repeat=4))
        assert (projected==f)==cylinder
        # Right-unit equality supplies the same role condition on equal domains.
        assert (stages(s,f,lift(d,identity))==f)==cylinder

# The readout/dependency equivalence also covers arbitrary coupled support.
for s in subsets(list(product(range(2),repeat=3))):
    for f in subsets(sorted(s)):
        for projection in [lambda p: p[:2],lambda p: p[0],lambda p: p[0]^p[1]^p[2]]:
            assert {p for p in s if projection(p) in {projection(v) for v in s}}==s
            values={projection(p) for p in f}
            saturated={p for p in s if projection(p) in values}
            dependency=all((a in f)==(b in f) for a in s for b in s if projection(a)==projection(b))
            assert (saturated==f)==dependency
            counts['coupled_readout_checks']+=1

# Full support does not make an arbitrary Event into a binary relation.
d={0,1};s=set(product(d,repeat=3));f={p for p in s if p[2]==1}
identity=lift(d,{(v,v) for v in d})
out=stages(s,f,identity)
assert (0,1,0) in out and (0,1,0) not in f
assert out!=f
copied={(0,0,0),(1,1,1)}
assert set(product({p[0] for p in copied},{p[1] for p in copied},{p[2] for p in copied}))!=copied
record=dict(passed=True,cases=counts,seed=20260918,
    counterexample=dict(support=sorted(s),event=sorted(f),right_identity=sorted(out),witness=[0,1,0]),
    full_fixed_nonproduct_support=sorted(copied),
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    boundary='Independent finite sets and matrix laws; all subsets of four presentation codes as legal domains. Exhaustive relation triples/events for domains of size at most two, selected samples for larger domains. General readout/dependency equivalence checks all supported predicates on all supports of three bits and three readouts. Does not test native Rust admission or environment-indexed execution.')
(LAB/'results/legal-relations-reference.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record,indent=2))
