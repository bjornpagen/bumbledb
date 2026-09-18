"""Finite set oracle for exact fusion of clipped maps and projection."""
from pathlib import Path
import hashlib,json,itertools

LAB=Path(__file__).resolve().parent
counts={};failures={}
def inverse(p):
    q=[0]*len(p)
    for i,j in enumerate(p):q[j]=i
    return tuple(q)
def source(w,p):return sum(((w>>j)&1)<<i for i,j in enumerate(p))
def rename(v,p):return sum(1<<w for w in range(1<<len(p)) if v>>source(w,p)&1)
def project(v,m,n):
    witnesses={w&~m for w in range(n) if v>>w&1}
    return sum(1<<w for w in range(n) if w&~m in witnesses)

for bits in [2,3]:
    n=1<<bits
    maps=[tuple(range(bits)),tuple(reversed(range(bits)))]
    if bits==3:maps.append((1,2,0))
    count=0
    for s in range(1,1<<n):
        if bits==2:
            bank=[v for v in range(1<<n) if v&~s==0]
            pairs=itertools.product(bank,repeat=2)
        else:
            bank=[0,s,s&0x96,s&0x3c,s&0xa5]
            pairs=[(bank[a],bank[b]) for a,b in [(0,4),(1,3),(2,4),(3,2),(4,1)]]
        for a,b in pairs:
            for am,bm,om in itertools.product(maps,repeat=3):
                for mask in range(4) if bits==2 else [0,1,5,7]:
                    # Reference evaluates the original scoped stages.
                    pa=s&rename(a,am);pb=s&rename(b,bm)
                    expected=s&rename(s&project(pa&pb,mask,n),om)
                    # Candidate absorbs common support into A via inverse map.
                    guarded_a=a&rename(s,inverse(am))
                    actual=s&rename(s,om)&rename(project(rename(guarded_a,am)&rename(b,bm),mask,n),om)
                    assert actual==expected,(s,a,b,am,bm,om,mask)
                    wrong={
                        'missing_witness_gate':s&rename(s&project(rename(a,am)&rename(b,bm),mask,n),om),
                        'missing_output_gate':s&rename(project(pa&pb,mask,n),om),
                    }
                    for key,value in wrong.items():
                        if value!=expected and key not in failures:
                            failures[key]=dict(bits=bits,support=s,a=a,b=b,am=am,bm=bm,output=om,mask=mask,expected=expected,wrong=value)
                    count+=1
    counts[bits]=count
assert len(failures)==2
result=dict(passed=True,cases=counts,counterexamples=failures,
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    boundary='Finite set identity; independent of Rust and not a timing result. Two-bit cases exhaust distinct supported operand pairs. Three-bit cases test selected pairs on every nonempty support, including non-involutive maps.')
(LAB/'results/scoped-reference.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
