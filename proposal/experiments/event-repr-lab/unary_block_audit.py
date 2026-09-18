"""Frozen block-kernel evidence. Does not build, time, or alter snapshots."""
from pathlib import Path
import itertools,json
from prepare import LAB,ROOT
from unary_audit import read,digest,marked

def verify():
    base=LAB/'results'
    expected=dict(passed=True,binary=25165824,projection=49152,maps=178176,
                  completed_projection=49152,orders=6,modes=2,kernels=2)
    checks=[]
    for tag in ['block','block-deep']:
        folder=base/f'unary-{tag}-src'
        for name,sha in read(folder/'manifest.json').items():assert digest(folder/name)==sha,(tag,name)
        d=read(base/f'unary-{tag}-check.json');checks.append(d)
        assert d['passed'] and d['records']==[expected]
        assert d['chain_records']==[dict(passed=True,world_checks=19200,max_dimensions=61)]
        assert digest(folder/'unary_check.py')==d['checker_sha256']
        for name,sha in d['source_hashes'].items():assert digest(folder/name)==sha
        log=(base/d['log']).read_text()
        for marker,key in [('UNARY_CHECK ','records'),('UNARY_CHAIN_CHECK ','chain_records'),('UNARY_LONG_SHAPE ','long_shapes')]:
            assert marked(log,marker)==d[key]
        assert len(d['long_shapes'])==8
        assert {(r['mode'],r['kernel'],r['reverse']) for r in d['long_shapes']}==set(itertools.product(['Plain','Chains'],['Steps','Blocks'],[False,True]))
        for r in d['long_shapes']:
            assert r['passed'] and r['dimensions']==61 and r['constructed_records']==123
            assert (r['compacted_records'],r['compacted_bytes'])==((62,4108) if r['mode']=='Plain' else (10,1282))
    for name in ['Cargo.toml','Cargo.lock','src/lib.rs','src/main.rs']:
        key='unary-prototype/'+name
        assert checks[0]['source_hashes'][key]==checks[1]['source_hashes'][key]
    deep=checks[1]
    assert deep['multi_records']==[dict(passed=True,four_axis_masks=128,visible_worlds=16384,hidden_assignments_per_world=16)]
    assert marked((base/deep['log']).read_text(),'UNARY_MULTI_CHECK ')==deep['multi_records']
    d=read(base/'unary-block-shapes.json');folder=base/'unary-block-src'
    assert d['passed'] and d['semantic_failures']==0
    for name,sha in d['source_hashes'].items():assert digest(folder/name)==sha
    assert digest(folder/'unary_shapes.py')==d['checker_sha256']
    assert digest(folder/'event-unary-prototype')==d['binary_sha256']
    assert (base/d['build_log']).is_file()
    jobs=set(itertools.product(['composition','coup','bilinear'],['plain','chains'],['bit-major','face-major'],['keep','compact'],['steps','blocks']))
    assert len(d['runs'])==48 and {tuple(r['job']) for r in d['runs']}==jobs
    old={tuple(r['job']):r for r in read(base/'unary-initial-shapes.json')['runs']}
    rows={}
    for run in d['runs']:
        job=tuple(run['job']);assert run['status']=='passed' and len(run['rows'])==1
        r=run['rows'][0];rows[job]=r;log=(base/run['log']).read_text()
        assert marked(log,'UNARY_SHAPE ')==run['rows']
        assert marked(log,'UNARY_PHASE ')==run['phases']
        assert [r[k] for k in ['shape','mode','layout','input_policy','kernel']]==list(job)
        assert r['passed'] and r['compacted_records']==r['live_union']+1<=r['final_records']
        assert r['input_records']==run['phases'][1]['records']
        assert r['input_bytes']==run['phases'][1]['bytes']
        if job[-1]=='steps':
            assert {k:v for k,v in r.items() if k!='kernel'}==old[job[:-1]]['rows'][0]
            assert run['phases']==old[job[:-1]]['phases']
    for job in old:
        a=rows[(*job,'steps')];b=rows[(*job,'blocks')]
        for field in ['dimensions','inputs','outputs','input_records','input_reachable','input_bytes',
                      'output_reachable','live_union','checked_world_answers','checksum','compacted_records','compacted_bytes']:
            assert a[field]==b[field],(job,field)
    reading=read(base/'unary-block-reading.json')
    for name,sha in reading['files'].items():assert digest(ROOT/name)==sha
    lean=read(base/'lean-check.json');proof=next(f for f in lean['files'] if f['file']=='UnaryProjection.lean')
    assert proof['passed'] and len(proof['checked_axioms'])==11
    assert sum(not a for a in proof['checked_axioms'].values())==7
    assert all(set(a)<={'propext'} for a in proof['checked_axioms'].values())
    assert digest(LAB/'lean/UnaryProjection.lean')==lean['source_hashes']['lean/UnaryProjection.lean']
    return dict(passed=True,structural_processes=48,unchanged_step_controls=24,matched_kernel_pairs=24,
        exhaustive_checks=expected,multi_axis_checks=deep['multi_records'],lean_reports=11,axiom_free_reports=7,
        checker_sha256=digest(Path(__file__)),boundary='Standalone structure and semantics, not native Free Join or latency evidence.')

if __name__=='__main__':
    result=verify();(LAB/'results/unary-block-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
