"""Audit frozen sparse shared-exit semantics and structural controls."""
from pathlib import Path
import itertools,json
from prepare import LAB,ROOT
from unary_audit import read,digest,marked

def verify():
    base=LAB/'results';folder=base/'range-initial-src'
    for name,sha in read(folder/'manifest.json').items():assert digest(folder/name)==sha,name
    d=read(base/'range-initial-check.json')
    expected=dict(passed=True,binary=25165824,projection=49152,maps=178176,
                  completed_projection=49152,orders=6,modes=2,kernels=2)
    assert d['passed'] and d['records']==[expected]
    assert d['chain_records']==[dict(passed=True,world_checks=19200,max_dimensions=61)]
    assert d['multi_records']==[dict(passed=True,four_axis_masks=128,visible_worlds=16384,hidden_assignments_per_world=16)]
    assert d['sparse_records']==[dict(passed=True,combinations=115200,projections=61440,cofactors=3840)]
    assert digest(folder/'range_check.py')==d['checker_sha256']
    for name,sha in d['source_hashes'].items():assert digest(folder/name)==sha
    log=(base/d['log']).read_text()
    for marker,key in [('RANGE_CHECK ','records'),('RANGE_CHAIN_CHECK ','chain_records'),
            ('RANGE_MULTI_CHECK ','multi_records'),('RANGE_SPARSE_CHECK ','sparse_records'),('RANGE_LONG_SHAPE ','long_shapes')]:
        assert marked(log,marker)==d[key]
    assert len(d['long_shapes'])==8
    assert {(r['mode'],r['kernel'],r['reverse']) for r in d['long_shapes']}==set(itertools.product(['Plain','Ranges'],['Steps','Groups'],[False,True]))
    for r in d['long_shapes']:
        assert r['passed'] and (r['dimensions'],r['constructed_records'],r['compacted_records'],r['compacted_bytes'])==(61,123,62,4108)
    d=read(base/'range-initial-shapes.json')
    assert d['passed'] and d['semantic_failures']==0
    assert digest(folder/'event-range-prototype')==d['binary_sha256']
    assert digest(folder/'range_shapes.py')==d['checker_sha256']
    for name,sha in d['source_hashes'].items():assert digest(folder/name)==sha
    assert (base/d['build_log']).is_file()
    jobs=set(itertools.product(['composition','coup','bilinear','shared-exit'],['plain','ranges'],['bit-major','face-major'],['keep','compact'],['steps','groups']))
    assert len(d['runs'])==64 and {tuple(r['job']) for r in d['runs']}==jobs
    old={tuple(r['job']):r for r in read(base/'unary-initial-shapes.json')['runs']}
    rows={};controls=0
    strength_checksum=0
    for tail in range(16):
        l=[bool(m>>tail&1) for m in [0xa639,0x91ba]]
        h=[bool(m>>tail&1) for m in [0x6b97,0xc8e5]]
        lc=l[0] and l[1];hc=h[0] and h[1];lx=l[0]^l[1];hx=h[0]^h[1]
        counts=[65535*hx+lx,65535*hc+lc,65532*hc+4*(lc or hc),65532*hx+4*(lx and hx)]
        strength_checksum+=sum((i+1)*n for i,n in enumerate(counts))
    for run in d['runs']:
        job=tuple(run['job']);assert run['status']=='passed' and len(run['rows'])==1
        r=run['rows'][0];rows[job]=r;log=(base/run['log']).read_text()
        assert marked(log,'RANGE_SHAPE ')==run['rows']
        assert marked(log,'RANGE_PHASE ')==run['phases']
        assert [r[k] for k in ['shape','mode','layout','input_policy','kernel']]==list(job)
        assert r['passed'] and r['compacted_records']==r['live_union']+1<=r['final_records']
        assert r['input_records']==run['phases'][1]['records'] and r['input_bytes']==run['phases'][1]['bytes']
        if job[3]=='compact':assert r['input_records']==r['input_reachable']+1
        else:assert r['input_records']==run['phases'][0]['records']
        dimensions,inputs,outputs,answers,checksum={
            'composition':(12,2,4,16384,3072),'coup':(16,36,64,4194304,5691950),
            'bilinear':(20,2,4,4194304,3469056),'shared-exit':(20,2,4,4194304,strength_checksum)}[r['shape']]
        for k,v in [('dimensions',dimensions),('inputs',inputs),('outputs',outputs),('checked_world_answers',answers),('checksum',checksum)]:assert r[k]==v
        if job[1]=='plain' or job[-1]=='steps':assert r['query_grouped_splits']==r['query_grouped_axes']==0
        if job[1]=='plain' and job[-1]=='steps' and job[0]!='shared-exit':
            assert all(r[k]==v for k,v in old[job[:-1]]['rows'][0].items());controls+=1
    pairs=0
    for job in jobs:
        if job[-1]!='steps':continue
        a=rows[job];b=rows[(*job[:-1],'groups')]
        for field in ['dimensions','inputs','outputs','input_records','input_reachable','input_bytes',
                      'output_reachable','live_union','checked_world_answers','checksum','compacted_records','compacted_bytes']:
            assert a[field]==b[field],(job,field)
        pairs+=1
    reading=read(base/'unary-block-reading.json')
    for name,sha in reading['files'].items():assert digest(ROOT/name)==sha
    lean=read(base/'lean-check.json');proof=next(f for f in lean['files'] if f['file']=='SharedExit.lean')
    assert proof['passed'] and len(proof['checked_axioms'])==10
    assert sum(not a for a in proof['checked_axioms'].values())==7
    assert all(set(a)<={'propext','Quot.sound'} for a in proof['checked_axioms'].values())
    assert digest(LAB/'lean/SharedExit.lean')==lean['source_hashes']['lean/SharedExit.lean']
    return dict(passed=True,structural_processes=64,unchanged_plain_controls=controls,matched_kernel_pairs=pairs,
        exhaustive_checks=expected,sparse_checks=read(base/'range-initial-check.json')['sparse_records'],
        lean_reports=10,axiom_free_reports=7,checker_sha256=digest(Path(__file__)),
        boundary='Raw structure, algebra and collection checks. Native carrier admission and Free Join comparison remain required.')

if __name__=='__main__':
    result=verify();(LAB/'results/range-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
