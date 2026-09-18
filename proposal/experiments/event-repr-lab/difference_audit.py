"""Verify isolated difference/counting evidence without compiling or timing."""
from pathlib import Path
import hashlib, itertools, json
from prepare import LAB, ROOT

def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p): return json.loads(p.read_text())
def marked(log, prefix):
    return [json.loads(s.split(prefix,1)[1]) for s in log.splitlines() if prefix in s]

def verify():
    results=LAB/'results'
    for tag in ['initial','kernel','bilinear','counting']:
        folder=results/f'difference-{tag}-src'
        for name,sha in read(folder/'manifest.json').items(): assert digest(folder/name)==sha,(tag,name)
    for name,sha in read(results/'difference-binaries/manifest.json').items():
        assert digest(results/'difference-binaries'/name)==sha

    def sources(record,tag,checker):
        folder=results/f'difference-{tag}-src'
        for name,sha in record['source_hashes'].items(): assert digest(folder/name)==sha,(tag,name)
        assert digest(folder/checker)==record['checker_sha256']

    checks={}
    for file,tag,multiplier in [('difference-initial-check.json','initial',1),
        ('difference-cofactor-check.json','kernel',2),('difference-counting-check.json','counting',2)]:
        r=read(results/file);sources(r,tag,'difference_check.py')
        assert r['passed'] and len(r['records'])==1
        log=(results/r['log']).read_text()
        assert marked(log,'DIFFERENCE_CHECK ')==r['records']
        assert 'test result: FAILED' not in log
        a=r['records'][0]
        for key,value in [('orders',6),('basis_schedules',4),('functions_per_manager',256),
            ('binary_cases',25165824*multiplier),('projection_cases',49152*multiplier),
            ('map_cases',178176*multiplier),('completed_projection_cases',49152*multiplier)]:
            assert a[key]==value,(file,key)
        if tag=='counting':
            assert marked(log,'DIFFERENCE_COUNT_CHECK ')==r['count_records']
            assert r['count_records']==[dict(passed=True,sparse_polynomials=1536,uniform_function_counts=12288,symbolic_width=61)]
        checks[tag]=a

    census={};screens={}
    for file,tag,shapes,expected in [
        ('difference-shapes.json','initial',['composition','coup'],(10,6)),
        ('difference-kernel-shapes.json','kernel',['composition','coup'],(26,6)),
        ('difference-bilinear-shapes.json','bilinear',['bilinear'],(16,0))]:
        r=read(results/file);sources(r,tag,'difference_shapes.py')
        jobspace=list(itertools.product(shapes,['shannon','positive','negative','mixed'],['face-major','bit-major']))
        if tag!='initial': jobspace=[(*j,k) for j in jobspace for k in ['coefficients','cofactors']]
        assert len(r['runs'])==len(jobspace)
        assert {tuple(j['job']) for j in r['runs']}==set(jobspace)
        passes=caps=0;rows={};phases={}
        for run in r['runs']:
            job=tuple(run['job']);log=(results/run['log']).read_text()
            assert marked(log,'DIFFERENCE_SHAPE ')==run['rows']
            if tag!='initial':
                assert marked(log,'DIFFERENCE_PHASE ')==run['phases']
                assert run['phases'][0]['phase']=='inputs_verified'
                phases[job]=run['phases']
            if run['status']!='passed':
                assert run['status']==('failed' if tag=='initial' else 'resource_cap')
                assert not run['rows'] and 'RESOURCE_CAP: coefficient nodes' in log
                assert job[0]=='coup' and job[1]!='shannon'
                if tag!='initial':
                    assert job[3]=='coefficients'
                    groups=[p['completed_groups'] for p in run['phases'] if p['phase']=='group']
                    assert groups==([0,8,16] if job[1:3]==('mixed','face-major') else [0])
                caps+=1;continue
            assert len(run['rows'])==1 and run['rows'][0]['passed']
            a=run['rows'][0];rows[job]=a;passes+=1
            assert [a['shape'],a['basis'],a['layout']]==list(job[:3])
            if tag!='initial': assert a['kernel']==job[3]
            dimensions,inputs,outputs,answers,checksum={
                'composition':(12,2,4,16384,3072),
                'coup':(16,36,64,4194304,5691950),
                'bilinear':(20,2,4,4194304,3469056)}[a['shape']]
            for key,value in [('dimensions',dimensions),('inputs',inputs),('outputs',outputs),
                ('checked_world_answers',answers),('checksum',checksum)]: assert a[key]==value,(file,key,a)
            assert a['output_reachable']<=a['live_union']<a['final_records']
            if tag!='initial':
                assert a['input_records']==run['phases'][0]['records']
                assert a['input_bytes']==run['phases'][0]['bytes']
        assert (passes,caps)==expected
        assert r['passed']==(caps==0)
        if tag!='initial':
            for shape,basis,layout in itertools.product(shapes,['shannon','positive','negative','mixed'],['face-major','bit-major']):
                akey=(shape,basis,layout,'coefficients');bkey=(shape,basis,layout,'cofactors')
                assert phases[akey][0]==phases[bkey][0]
                if akey not in rows: continue
                a,b=rows[akey],rows[bkey]
                for field in ['input_records','input_reachable','input_bytes','output_reachable','live_union','checksum','checked_world_answers']:
                    assert a[field]==b[field],(tag,akey,field)
                if basis=='shannon':
                    assert {k:v for k,v in a.items() if k!='kernel'}=={k:v for k,v in b.items() if k!='kernel'}
        if tag=='kernel':
            for key,a in rows.items():
                old=census['initial'].get(key[:3])
                if key[3]=='coefficients' and old:
                    assert {k:v for k,v in a.items() if k!='kernel'}==old
        if tag=='bilinear':
            binary=results/'difference-binaries/event-difference-prototype'
            assert digest(binary)==r['binary_sha256']
        census[tag]=rows
        screens[tag]=dict(processes=len(r['runs']),completed=passes,node_caps=caps,semantic_failures=0)

    r=read(results/'difference-counting-shapes.json');sources(r,'counting','counting_shapes.py')
    assert (results/r['build_log']).is_file()
    assert digest(results/'difference-binaries/constraint_counting')==r['binary_sha256']
    expected={('small',)}|set(itertools.product(map(str,[4,8,12,16,24]),['forward','reverse'],['coefficients','cofactors']))
    assert len(r['runs'])==len(expected) and {tuple(j['job']) for j in r['runs']}==expected
    countrows={};countphases={};passes=caps=0
    for run in r['runs']:
        job=tuple(run['job']);log=(results/run['log']).read_text()
        marker='COUNTING_SMALL ' if job==('small',) else 'COUNTING_SHAPE '
        assert marked(log,marker)==run['rows']
        assert marked(log,'COUNTING_PHASE ')==run['phases']
        if job!=('small',):
            assert len(run['phases'])==1 and run['phases'][0]['phase']=='input_verified'
            countphases[job]=run['phases'][0]
        if run['status']!='passed':
            assert job[0]=='24' and run['status']=='resource_cap'
            assert not run['rows'] and 'RESOURCE_CAP: coefficient nodes' in log
            caps+=1;continue
        passes+=1;assert len(run['rows'])==1 and run['rows'][0]['passed']
        a=run['rows'][0]
        if job==('small',):
            assert a==dict(passed=True,circuits_and_outputs=197,manager_cases=788,orders=2,kernels=2)
            continue
        g=int(job[0]);countrows[job]=a
        assert a['gates']==g and a['dimensions']==2*g+5
        assert a['reverse']==(job[1]=='reverse') and a['kernel'].lower()==job[2]
        assert a['terms']<=3*g+2
        assert a['input_nodes']<=a['dimensions']*a['terms']+1
        assert a['circuit_solutions']=={4:4,8:0,12:6,16:2}[g]
        assert a['polynomial_ones']==(1<<(a['dimensions']-1))-(a['circuit_solutions']<<g)
        assert a['input_nodes']==run['phases'][0]['nodes'] and a['input_bytes']==run['phases'][0]['bytes']
    assert (passes,caps)==(17,4) and not r['passed'] and r['semantic_failures']==0
    for g,order in itertools.product(map(str,[4,8,12,16,24]),['forward','reverse']):
        akey=(g,order,'coefficients');bkey=(g,order,'cofactors')
        assert countphases[akey]==countphases[bkey]
        if g=='24': continue
        assert {k:v for k,v in countrows[akey].items() if k!='kernel'}=={k:v for k,v in countrows[bkey].items() if k!='kernel'}
    screens['counting']=dict(processes=21,completed=17,node_caps=4,semantic_failures=0)

    papers=[];folder=ROOT/'proposal/research/difference-decomposition'
    for manifest in ['sources.json','counting-sources.json']:
        for source in read(folder/manifest):
            assert digest(folder/source['file'])==source['sha256']
            assert digest(folder/source['text_file'])==source['text_sha256']
            assert 'in progress' not in source['reading_scope']
            papers.append(source['arxiv'])
    lean=read(results/'lean-check.json')
    for file,count in [('Differential.lean',11),('ConstraintCounting.lean',7)]:
        checked=next(f for f in lean['files'] if f['file']==file)
        assert checked['passed'] and len(checked['checked_axioms'])==count
        assert all(set(a)<={'propext','Quot.sound'} for a in checked['checked_axioms'].values())
        assert digest(LAB/'lean'/file)==lean['source_hashes']['lean/'+file]
    return dict(passed=True,checks=checks,screens=screens,primary_sources=papers,
        checker_sha256=digest(Path(__file__)),
        boundary='Frozen standalone structural evidence and denotational Lean reports; no native Free Join or latency ranking. Initial failed statuses are reclassified from retained node-cap logs without altering raw records.')

if __name__=='__main__':
    output=verify()
    (LAB/'results/difference-audit.json').write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps(output,indent=2))
