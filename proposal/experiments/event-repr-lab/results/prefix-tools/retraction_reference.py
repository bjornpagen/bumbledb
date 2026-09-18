"""Independent finite sets: fixed completion, fibred relations, clipped fallback."""
from pathlib import Path
from itertools import product, permutations
import hashlib, json, random
LAB=Path(__file__).resolve().parent
rng=random.Random(20260918)
cases=dict(decoders=0,events=0,boolean=0,projections=0,maps=0,compositions=0)
for ds in [({0,1,2},), ({1,3},), ({0,2},{1}), (set(),{1,2}), ({1},set())]:
    size=4; envs=len(ds); n=envs*64; worlds=set(range(n))
    split=lambda w:(w//64,w%4,(w//4)%4,(w//16)%4)
    def join(e,x,y,z):return e*64+x+y*4+z*16
    support={w for w in worlds if all(v in ds[split(w)[0]] for v in split(w)[1:])}
    anchor_env=next(e for e,d in enumerate(ds) if d)
    def decode(w):
        e,x,y,z=split(w);e=e if ds[e] else anchor_env
        return join(e,*(v if v in ds[e] else min(ds[e]) for v in (x,y,z)))
    assert {decode(w) for w in worlds}==support
    assert all(decode(w)==w for w in support)
    encode=lambda a:{w for w in worlds if decode(w) in a}
    bank=[set(),support]+[{w for w in support if rng.getrandbits(1)} for _ in range(24)]
    for a in bank:
        aa=encode(a); cases['events']+=1
        assert aa & support==a and encode(aa)==aa
        assert bool(aa)==bool(a) and encode(support-a)==worlds-aa
        for b in bank:
            bb=encode(b)
            assert (aa==bb)==(a==b)
            for op in range(16):
                truth=lambda w,p,q:bool(op>>(2*(w in p)+(w in q))&1)
                assert encode({w for w in support if truth(w,a,b)})=={w for w in worlds if truth(w,aa,bb)}
                cases['boolean']+=1
        for mask in range(n.bit_length()-1):
            mask=1<<mask
            raw={w for w in worlds if any((v&~mask)==(w&~mask) for v in a)}
            expected=raw&support
            assert encode(raw)==encode(expected)
            cases['projections']+=1
        for face in range(3):
            mask=3<<(face*2)
            raw={w for w in worlds if any((v&~mask)==(w&~mask) for v in aa)}
            expected={w for w in support if any((v&~mask)==(w&~mask) for v in a)}
            assert raw==encode(expected)
        for perm in permutations(range(3)):
            def f(w):
                e,*faces=split(w);return join(e,*(faces[i] for i in perm))
            assert all(decode(f(w))==f(decode(w)) for w in worlds)
            assert {w for w in worlds if f(w) in aa}==encode({w for w in support if f(w) in a})
            cases['maps']+=1
    relations=[{(e,x,y) for e,d in enumerate(ds) for x in d for y in d if rng.getrandbits(1)} for _ in range(20)]
    for r,q in product(relations,repeat=2):
        expected={(e,x,z) for e,d in enumerate(ds) for x in d for z in d if any((e,x,y) in r and (e,y,z) in q for y in d)}
        for w in worlds:
            e,x,y,z=split(decode(w))
            actual=any((e,x,split(decode(join(e,0,t,0)))[2]) in r and (e,split(decode(join(e,0,t,0)))[2],z) in q for t in range(size))
            assert actual==((e,x,z) in expected)
        cases['compositions']+=1
    cases['decoders']+=1
# Deliberately wrong optimizations, with retained witnesses.
rho=lambda v:v if v<3 else 0
assert len({v for v in range(4) if rho(v)==0})==2
assert len({v for v in range(3) if v==0})==1
assert any(rho(v)==0 for v in [2,3]) and not any(v==0 for v in [2])
repair=lambda v:v if v in {1,2} else 1
swap=lambda v:((v&1)<<1)|(v>>1)
assert {swap(v) for v in [1,2]}=={1,2}
assert repair(swap(0))!=swap(repair(0))
out=dict(passed=True,cases=cases,counterexamples=dict(alias_population=[2,1],partial_projection=[2,3],preserving_noncommuting=dict(support=[1,2],witness=0)),checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
(LAB/'results/retraction-reference.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
