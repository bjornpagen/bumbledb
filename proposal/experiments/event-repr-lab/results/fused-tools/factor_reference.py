"""Independent finite product-fibre counts, including empty environments and aliases."""
from pathlib import Path
import hashlib,itertools,json
LAB=Path(__file__).resolve().parent
counts=dict(domains=0,event_cases=0,fibre_replacements=0)
for width in [1,2]:
    size=1<<width
    subsets=[[v for v in range(size) if mask>>v&1] for mask in range(1<<size)]
    configurations=[(s,) for s in subsets if s]+[p for p in itertools.product(subsets,repeat=2) if any(p)]
    for domains in configurations:
        counts['domains']+=1
        raw=list(itertools.product(range(len(domains)),range(size),range(size),range(size)))
        legal=[w for w in raw if all(v in domains[w[0]] for v in w[1:])]
        anchor=next(e for e,d in enumerate(domains) if d)
        for prefix in [False,True]:
            def repair(e,v):
                d=domains[e]
                return min(d,key=lambda s:v^s) if prefix else v if v in d else min(d)
            decoded=[]
            for w in raw:
                e=w[0] if domains[w[0]] else anchor
                decoded.append((e,*(repair(e,v) for v in w[1:])))
            for mask in range(8):
                selected=[i for i in range(3) if mask>>i&1]
                omitted=3-len(selected)
                divisor=size**omitted
                for seed in range(4):
                    def event(w):
                        key=w[0]+sum((w[i+1]<<(width*i+1)) for i in selected)
                        return (key*37+seed*19+(key*key>>1))%11 < 5
                    complete=[event(w) for w in decoded]
                    actual=sum(event(w) for w in legal)
                    total=0
                    for e,d in enumerate(domains):
                        if not d:continue
                        intermediate=sum(a and w[0]==e and all(w[i+1] in d for i in selected) for w,a in zip(raw,complete))
                        assert intermediate%divisor==0
                        total+=intermediate//divisor*len(d)**omitted
                        counts['fibre_replacements']+=1
                    assert total==actual
                    counts['event_cases']+=1
# E ignores z, but support copies x to z: domain marginals alone are insufficient.
support=[w for w in itertools.product(range(2),repeat=3) if w[0]==w[2]]
actual=sum(x==1 for x,y,z in support)
wrong=sum(x==1 for x,y in itertools.product(range(2),repeat=2))*2
assert (actual,wrong)==(2,4)
result=dict(passed=True,cases=counts,coupled_counterexample=dict(actual=actual,invalid_product_shortcut=wrong),checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),boundary='Finite exact reference; no performance claim or probability-law independence inferred.')
(LAB/'results/factor-reference.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
