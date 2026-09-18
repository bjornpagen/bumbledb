"""Independent finite-cube proof that the native axes fixture exercises alignment."""
from pathlib import Path
import hashlib, itertools, json

LAB=Path(__file__).resolve().parent
bank=[]
for i in range(48):
    axis=i%16
    mode=i//16
    axes=(axis,) if mode==0 else (axis,(axis+(5 if mode==1 else 9))%16)
    bank.append((mode,axes,False))
bank += [(mode,axes,True) for mode,axes,_ in bank]
bank += [(3,(),False),(3,(),True)]

def value(spec,world):
    mode,axes,flip=spec
    bits=[bool(world>>v&1) for v in axes]
    result=bits[0] if mode==0 else all(bits) if mode==1 else any(bits) if mode==2 else False
    return result != flip

def worlds(axes):
    for values in itertools.product([False,True],repeat=len(axes)):
        yield sum(int(bit)<<axis for bit,axis in zip(values,axes))

# Prove that the declared axes are exactly the Boolean dependencies. Every
# declared coordinate has differing cofactors; omitted coordinates are not read.
for spec in bank:
    for axis in spec[1]:
        assert any(value(spec,w)!=value(spec,w^(1<<axis)) for w in worlds(spec[1]))

pairs={}
same_mask_pairs=0
for a,A in enumerate(bank):
    for b,B in enumerate(bank):
        axes=sorted(set(A[1])|set(B[1]))
        signature=0
        for w in worlds(axes):
            signature |= 1 << (2*value(A,w)+value(B,w))
        different=set(A[1])!=set(B[1])
        if not different:
            same_mask_pairs+=1
            assert signature!=15
        pairs[a,b]=(signature,different)

records=[]
for shape in ['triangle','clover']:
    histogram=[0]*16
    group_counts={name:[0]*64 for name in ['included','disjoint','overlap']}
    alignment_bindings=0
    transformations=0
    unique={}
    for g in range(64):
        for j in range(4):
            a=(g*17+j*13)%len(bank)
            middle=(g+j)%64 if shape=='triangle' else g
            for k in range(4):
                b=(middle*17+k*13+7)%len(bank)
                multiplicity=1 if shape=='triangle' else 4
                sig,different=pairs[a,b]
                histogram[sig]+=multiplicity
                keeps={'included':not bool(sig&4),'disjoint':not bool(sig&8),'overlap':bool(sig&8)}
                for name,keep in keeps.items():group_counts[name][g]+=multiplicity*keep
                if a<96 and b<96 and different:
                    # Both are proper. Distinct essential masks forbid equality
                    # and complement shortcuts. Their union has at most four
                    # axes: one local cube, no pinned-table cofactor, and exactly
                    # the symmetric difference's broadcasts in either word path.
                    steps=len(set(bank[a][1])^set(bank[b][1]))
                    assert 1<=steps<=4
                    alignment_bindings+=multiplicity
                    transformations+=multiplicity*steps
                    key=tuple(sorted((a%48,b%48)))
                    assert unique.setdefault(key,steps)==steps
    total=sum(histogram)
    assert histogram[0]==0 and histogram[15]>0
    assert total==(1024 if shape=='triangle' else 4096)
    assert all(0<sum(groups)<total for groups in group_counts.values())
    records.append(dict(shape=shape,rows=total,histogram=histogram,
                        accepted={name:sum(groups) for name,groups in group_counts.items()},
                        group_counts=group_counts,alignment_bindings=alignment_bindings,
                        uncached_broadcasts=transformations,normalized_alignment_pairs=len(unique),
                        first_cached_broadcasts=sum(unique.values())))

result=dict(passed=True,predicates=len(bank),same_mask_pairs=same_mask_pairs,
            pairs_checked=len(pairs),queries=records,
            source_hashes={name:hashlib.sha256((LAB/name).read_bytes()).hexdigest() for name in
                           ['src/lib.rs','src/native.rs','src/signature_bench.rs']},
            checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            boundary='Exact fixture/algorithm counts from finite cubes and controlled row topology. These are not allocator samples or timings. Native histograms and survivors must match separately.')
(LAB/'results/axes-reference.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['source_hashes','queries']},indent=2))
for r in records:print(r['shape'],r['rows'],'bindings;',r['alignment_bindings'],'require alignment;',r['uncached_broadcasts'],'uncached broadcasts;',r['normalized_alignment_pairs'],'normalized pairs;',r['first_cached_broadcasts'],'first cached broadcasts')
