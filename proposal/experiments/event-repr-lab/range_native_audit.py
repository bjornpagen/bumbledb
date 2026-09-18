"""Verify the isolated native range build, admission and same-binary join evidence."""
from pathlib import Path
import itertools,json,statistics
from prepare import LAB,ROOT
from unary_audit import read,digest,marked

def verify():
    base=LAB/'results';build=read(base/'build-range-native.json');snap=base/'range-native-src'
    for name,sha in read(snap/'manifest.json').items():assert digest(snap/name)==sha,name
    assert digest(snap/'bumbledb-range')==build['binary_sha256']
    for name,sha in build['source_sha256'].items():assert digest(snap/Path(name).name)==sha
    assert digest(snap/'range_native_build.py')==build['builder_sha256']
    for name,sha in build['production_manifest']['source_sha256'].items():assert digest(ROOT/name)==sha
    assert (base/build['log']).is_file()
    for tag in ['initial','fixed']:
        prior=base/f'range-admission-{tag}-src'
        for name,sha in read(prior/'manifest.json').items():assert digest(prior/name)==sha
        check=read(base/f'range-admission-{tag}.json')
        assert check['passed']==(tag=='fixed')
        for name,sha in check['lab_sources'].items():assert digest(prior/name)==sha
    admission=read(base/'range-admission-fixed.json')
    for name,sha in admission['lab_sources'].items():
        if Path(name).name!='lib.rs':assert sha==build['source_sha256'][name],name
    expected=[dict(passed=True,candidate=c,coefficient_cases=30,symbolic_dimensions=61)
        for c in ['range-shannon','range-step','range-group']]
    assert marked((base/admission['log']).read_text(),'RANGE_ADMISSION ')==expected
    totals={};datasets={}
    for phase,processes in [('correctness',1),('admission',12),('join',14)]:
        data=read(base/f'range-native-{phase}.json');datasets[phase]=data
        assert data['passed'] and data['phase']==phase and len(data['runs'])==processes
        assert data['build_metadata']==build
        assert data['runner_sha256']==digest(snap/'range_native_sweep.py')
        assert data['execute_sha256']==digest(snap/'run.py')
        for run in data['runs']:
            assert run['status']=='passed';log=(base/run['log']).read_text()
            raw=marked(log,'EVENT_LAB ');assert len(raw)==len(run['rows'])
            for parsed,row in zip(raw,run['rows']):
                assert all(row[k]==v for k,v in parsed.items())
                assert parsed.get('verified',parsed.get('passed',False))
            if phase=='correctness':assert marked(log,'RANGE_ADMISSION ')==expected
        totals[phase]=sum(len(run['rows']) for run in data['runs'])
    actual={(r['job']['EVENT_LAB_CANDIDATE'],r['job']['EVENT_LAB_LANE']) for r in datasets['admission']['runs']}
    assert actual==set(itertools.product(['range-shannon','range-step','range-group'],['owned','symbolic-relations','transport','laws']))
    joins=datasets['join'];assert joins['trials']>=7
    rows={(r['candidate'],r['scenario'],r['shape'],r['memo']):r for run in joins['runs'] for r in run['rows']}
    candidates=['range-shannon','range-step','range-group','packed512','dense','essential512','retraction512']
    cases=list(itertools.product(['coup_4290','coup_product_65536'],['triangle','clover'],[False,True]))
    assert set(rows)=={(c,*case) for c,case in itertools.product(candidates,cases)}
    for case in cases:
        ref=rows[('range-shannon',*case)]
        for c in candidates:
            r=rows[(c,*case)];assert r['kind']=='free_join'
            for field in ['checksum','rows','groups','worlds','admissible_worlds']:assert r[field]==ref[field]
            for field in ['fresh_s','warm_s','build_s','join_only_s']:assert len(r[field])==joins['trials']
    comparison=read(base/'range-native-comparison.json')
    assert comparison['passed'] and comparison['binary_sha256']==build['binary_sha256']
    assert comparison['input_sha256']==digest(base/'range-native-join.json')
    assert comparison['checker_sha256']==digest(snap/'range_native_analysis.py')
    assert len(comparison['comparisons'])==48
    for c in comparison['comparisons']:
        key=(c['scenario'],c['shape'],c['memo'])
        for side,candidate in [('group','range-group'),('control',c['baseline'])]:
            r=rows[(candidate,*key)]
            for field in ['fresh','warm','build','join_only']:
                assert c[side][field+'_ms']==statistics.median(r[field+'_s'])*1000
        assert c['fresh_ratio']==c['group']['fresh_ms']/c['control']['fresh_ms']
        assert c['warm_ratio']==c['group']['warm_ms']/c['control']['warm_ms']
    factor=read(base/'factorized-pack-lean.json');assert factor['passed'] and len(factor['theorems'])==6
    assert all(not a for a in factor['theorems'].values())
    assert digest(LAB/factor['source'])==factor['source_sha256']
    log=(base/factor['log']).read_text()
    for theorem in factor['theorems']:assert "'FactorizedPack."+theorem+"' does not depend on any axioms" in log
    oracle=read(base/'factorized-pack-oracle.json');assert oracle['passed'] and oracle['local_predicate_checks']==16384 and oracle['factorable_relations']==28
    assert digest(LAB/'join-factorization/oracle.py')==oracle['checker_sha256']
    for name,sha in oracle['source_hashes'].items():assert digest(ROOT/name)==sha
    return dict(passed=True,binary_sha256=build['binary_sha256'],phase_rows=totals,native_join_configurations=56,
        matched_comparisons=48,query_factorization_reports=6,checker_sha256=digest(Path(__file__)),
        boundary='Scoped/anchored native controls. Separate raw replay uses completed inputs. Factorized query execution is proved but not implemented.')

if __name__=='__main__':
    result=verify();(LAB/'results/range-native-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
