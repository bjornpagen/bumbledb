"""Check compact-label structural evidence without compiling or benchmarking."""
from pathlib import Path
import hashlib,itertools,json
from prepare import LAB,ROOT

def read(p):return json.loads(p.read_text())
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def marked(log,tag):return [json.loads(s.split(tag,1)[1]) for s in log.splitlines() if tag in s]

def verify():
    base=LAB/'results'
    for tag in ['initial','long']:
        folder=base/f'unary-{tag}-src'
        for name,sha in read(folder/'manifest.json').items():assert digest(folder/name)==sha,(tag,name)
    expected=dict(passed=True,binary=12582912,projection=24576,maps=89088,completed_projection=24576,orders=6,modes=2)
    for name,tag in [('unary-initial-check.json','initial'),('unary-long-check.json','long')]:
        d=read(base/name);folder=base/f'unary-{tag}-src'
        assert d['passed'] and d['records']==[expected]
        assert d['chain_records']==[dict(passed=True,world_checks=9600,max_dimensions=61)]
        assert digest(folder/'unary_check.py')==d['checker_sha256']
        for path,sha in d['source_hashes'].items():assert digest(folder/path)==sha
        log=(base/d['log']).read_text()
        assert marked(log,'UNARY_CHECK ')==d['records']
        assert marked(log,'UNARY_CHAIN_CHECK ')==d['chain_records']
        if tag=='long':
            assert marked(log,'UNARY_LONG_SHAPE ')==d['long_shapes']
            assert len(d['long_shapes'])==4
            for row in d['long_shapes']:
                assert row['passed'] and row['dimensions']==61 and row['constructed_records']==123
                assert (row['compacted_records'],row['compacted_bytes'])==((62,4108) if row['mode']=='Plain' else (10,1282))
    d=read(base/'unary-initial-shapes.json');folder=base/'unary-initial-src'
    assert d['passed'] and d['semantic_failures']==0
    assert digest(folder/'unary_shapes.py')==d['checker_sha256']
    assert digest(folder/'event-unary-prototype')==d['binary_sha256']
    for name,sha in d['source_hashes'].items():assert digest(folder/name)==sha
    assert (base/d['build_log']).is_file()
    jobs=set(itertools.product(['composition','coup','bilinear'],['plain','chains'],['bit-major','face-major'],['keep','compact']))
    assert len(d['runs'])==24 and {tuple(r['job']) for r in d['runs']}==jobs
    rows={}
    for run in d['runs']:
        assert run['status']=='passed' and len(run['rows'])==1
        log=(base/run['log']).read_text();a=run['rows'][0];job=tuple(run['job']);rows[job]=a
        assert marked(log,'UNARY_SHAPE ')==run['rows']
        assert marked(log,'UNARY_PHASE ')==run['phases']
        assert [a[k] for k in ['shape','mode','layout','input_policy']]==list(job)
        assert run['phases'][0]['phase']=='inputs_constructed' and run['phases'][1]['phase']=='inputs_verified'
        assert a['input_records']==run['phases'][1]['records']
        assert a['input_bytes']==run['phases'][1]['bytes']
        assert a['compacted_records']==a['live_union']+1<=a['final_records']
        if a['input_policy']=='compact':assert a['input_records']==a['input_reachable']+1
        else:assert a['input_records']==run['phases'][0]['records']
        dimensions,inputs,outputs,answers,checksum={
            'composition':(12,2,4,16384,3072),'coup':(16,36,64,4194304,5691950),
            'bilinear':(20,2,4,4194304,3469056)}[a['shape']]
        for k,v in [('dimensions',dimensions),('inputs',inputs),('outputs',outputs),('checked_world_answers',answers),('checksum',checksum)]:assert a[k]==v
    pairs=0
    for shape,layout,policy in itertools.product(['composition','coup','bilinear'],['bit-major','face-major'],['keep','compact']):
        a=rows[(shape,'plain',layout,policy)];b=rows[(shape,'chains',layout,policy)]
        for field in ['final_records','final_bytes','query_apply_misses','query_cofactor_misses','query_projection_misses','checksum']:
            assert a[field]==b[field],(shape,layout,policy,field)
        assert b['compacted_records']<=a['compacted_records']
        pairs+=1
    for shape,layout,mode in itertools.product(['composition','coup','bilinear'],['bit-major','face-major'],['plain','chains']):
        a=rows[(shape,mode,layout,'keep')];b=rows[(shape,mode,layout,'compact')]
        for field in ['final_records','output_reachable','compacted_records','compacted_bytes','query_apply_misses','query_projection_misses']:
            assert a[field]==b[field],(shape,layout,mode,field)
    reading=read(base/'unary-reading.json')
    for name,sha in reading['files'].items():assert digest(ROOT/name)==sha
    lean=read(base/'lean-check.json');checked=next(f for f in lean['files'] if f['file']=='UnaryChains.lean')
    assert checked['passed'] and len(checked['checked_axioms'])==9
    assert sum(not a for a in checked['checked_axioms'].values())==6
    assert all(set(a)<={'propext','Quot.sound'} for a in checked['checked_axioms'].values())
    assert digest(LAB/'lean/UnaryChains.lean')==lean['source_hashes']['lean/UnaryChains.lean']
    return dict(passed=True,structural_processes=24,matched_layout_pairs=pairs,semantic_failures=0,
        exhaustive_checks=expected,long_chain_world_checks=9600,lean_reports=9,axiom_free_reports=6,
        reading=reading,checker_sha256=digest(Path(__file__)),
        boundary='Raw structural/collection evidence; no native Free Join, latency ranking, concurrent GC, common packet or designated-law contraction.')

if __name__=='__main__':
    result=verify();(LAB/'results/unary-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
