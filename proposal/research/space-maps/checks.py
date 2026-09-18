"""Independent finite checks of substitution/quantifier and law-preservation boundaries."""
from pathlib import Path
from fractions import Fraction as Q
import json

def members(mask,n): return {i for i in range(n) if mask>>i&1}
def saturate(support,event,hide):
    keys={w & ~hide for w in event}
    return {w for w in support if w & ~hide in keys}

def lift(event,target): return {w for w in target if (w&3) in event}
counts={'support_pairs':0,'extensions':0,'squares':0,'commuting_squares':0,'event_checks':0}
for sm in range(1,16):
    s=members(sm,4)
    for dm in range(1,256):
        d=members(dm,8)
        counts['support_pairs']+=1
        if {w&3 for w in d}!=s: continue
        counts['extensions']+=1
        for hide in [1,2,3]:
            counts['squares']+=1
            complete=all((old | (w&4)) in d for w in d for old in s if (old&~hide)==((w&3)&~hide))
            all_equal=True
            for em in range(16):
                e=members(em,4)&s
                left=lift(saturate(s,e,hide),d)
                right=saturate(d,lift(e,d),hide)
                assert right<=left
                all_equal &= left==right
                counts['event_checks']+=1
            assert all_equal==complete
            counts['commuting_squares']+=all_equal
# One old bit x, one copied bit y; encoding is x + 2*y.
s={0,1};d={0,3};e={1}
left={w for w in d if (w&1) in saturate(s,e,1)}
right=saturate(d,{w for w in d if (w&1) in e},1)
assert left=={0,3} and right=={3}
# Structural surjectivity does not preserve a specified law.
new=[Q(1,8),Q(3,8),Q(1,8),Q(3,8)]
assert sum(new)==1 and new[1]+new[3]==Q(3,4)!=Q(1,2)
# Full product support and equal marginals do not imply independent measurement.
correlated=[Q(3,8),Q(1,8),Q(1,8),Q(3,8)]
assert correlated[1]+correlated[3]==correlated[2]+correlated[3]==Q(1,2)
assert correlated[3]==Q(3,8)!=Q(1,4)
result={'passed':True,**counts,'copied_coordinate_counterexample':{'source':[0,1],'target':[0,3],'lift_after_hide':sorted(left),'hide_after_lift':sorted(right)},'structural_extension_changed_probability':['1/2','3/4'],'same_marginals_joint_probabilities':['1/4','3/8'],'scope':'Independent finite mathematics, not a native timing or a generalized Rust map implementation.'}
Path(__file__).with_name('checks.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
