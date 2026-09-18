"""Independent finite check of prefix membership FDs and XOR-nearest repair."""
from pathlib import Path
import hashlib,json
LAB=Path(__file__).resolve().parent
counts=dict(domains=0,decoder_worlds=0,events=0,dependency_equivalences=0)

def independent_on_prefix(worlds, membership, suffix):
    seen={}
    for w in worlds:
        p=w>>suffix
        if p in seen and seen[p]!=membership(w):return False
        seen[p]=membership(w)
    return True

for width in range(1,4):
    n=1<<width
    for support in range(1,1<<n):
        legal=[w for w in range(n) if support>>w&1]
        decoded=[]
        for w in range(n):
            prefix=0
            for bit in reversed(range(width)):
                chosen=prefix | (w & (1<<bit))
                if not any(v>>bit==chosen>>bit for v in legal):chosen^=1<<bit
                prefix=chosen
            # XOR ranking is an independent characterization, not an assumed metric.
            assert prefix==min(legal,key=lambda s:w^s)
            decoded.append(prefix)
            counts['decoder_worlds']+=1
        counts['domains']+=1
        for selector in range(1<<len(legal)):
            event={w for i,w in enumerate(legal) if selector>>i&1}
            counts['events']+=1
            for suffix in range(width+1):
                logical=independent_on_prefix(legal,lambda w:w in event,suffix)
                physical=independent_on_prefix(range(n),lambda w:decoded[w] in event,suffix)
                assert logical==physical
                counts['dependency_equivalences']+=1
result=dict(passed=True,cases=counts,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),boundary='Finite exact reference, not a performance measurement. Lean proves the dependency equivalence for arbitrary finite cube width. XOR-nearest is additionally checked here; it is not a separate Lean theorem.')
(LAB/'results/prefix-dependency-reference.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
