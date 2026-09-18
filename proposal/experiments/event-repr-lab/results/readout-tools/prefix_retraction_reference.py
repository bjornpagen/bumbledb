"""Unimplemented Rust candidate: repair prefixes using legal extendibility."""
from pathlib import Path
import json,hashlib
LAB=Path(__file__).resolve().parent
counts=dict(domains=0,events=0,suffix_projections=0,prefix_readouts=0)
counterexample=None
for dimensions in range(1,4):
 n=1<<dimensions; all_bits=n-1
 for support in range(1,1<<n):
  legal=[w for w in range(n) if support>>w&1]
  def repair(w):
   prefix=0
   for bit in reversed(range(dimensions)):
    selected=prefix | (w & (1<<bit))
    retained=all_bits ^ ((1<<bit)-1)
    if not any(v & retained == selected for v in legal):selected ^= 1<<bit
    prefix=selected
   return prefix
  decoded=[repair(w) for w in range(n)]
  assert set(decoded)==set(legal)
  assert all(decoded[w]==w for w in legal)
  counts['domains']+=1
  for k in range(dimensions+1):
   low=(1<<k)-1
   for x in range(n):
    image={decoded[v] for v in range(n) if v & ~low == x & ~low}
    wanted={v for v in legal if v & ~low == decoded[x] & ~low}
    assert image==wanted # Exact fibre image, not merely support preservation.
    counts['prefix_readouts']+=1
  for selector in range(1<<len(legal)):
   event={w for i,w in enumerate(legal) if selector>>i&1}
   completed={w for w in range(n) if decoded[w] in event}
   counts['events']+=1
   for k in range(dimensions+1):
    mask=(1<<k)-1
    direct={w for w in range(n) if any(v & ~mask == w & ~mask for v in completed)}
    supported={w for w in legal if any(v & ~mask == w & ~mask for v in event)}
    assert direct=={w for w in range(n) if decoded[w] in supported}
    counts['suffix_projections']+=1
   if counterexample is None:
    for mask in range(n):
     if mask&(mask+1)==0:continue
     direct={w for w in range(n) if any(v & ~mask == w & ~mask for v in completed)}
     supported={w for w in legal if any(v & ~mask == w & ~mask for v in event)}
     expected={w for w in range(n) if decoded[w] in supported}
     if direct!=expected:
      counterexample=dict(dimensions=dimensions,legal=legal,decoder=decoded,event=sorted(event),mask=mask,raw=sorted(direct),expected=sorted(expected));break
assert counterexample is not None
record=dict(passed=True,cases=counts,nonprefix_counterexample=counterexample,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),boundary='Exhaustive sets through three coordinates. No Rust implementation, native performance, or general algorithm proof is claimed.')
(LAB/'results/prefix-retraction-reference.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record,indent=2))
