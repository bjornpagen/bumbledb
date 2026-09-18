"""Retained native relational Pack, separator and proof evidence."""
from pathlib import Path
import json
from prepare import LAB,ROOT
from unary_audit import read,digest,marked
from relation_pack_analysis import analyze,CANDIDATES

def verify():
    base=LAB/'results';build=read(base/'build-relation-pack.json');snap=base/'relation-pack-src'
    for n,sha in read(snap/'manifest.json').items():assert digest(snap/n)==sha,n
    assert digest(snap/'bumbledb-relation-pack')==build['binary_sha256']
    for n,sha in build['source_sha256'].items():assert digest(snap/Path(n).name)==digest(LAB/n)==sha,n
    assert digest(snap/'relation_pack_build.py')==build['builder_sha256']
    for n,sha in build['production_manifest']['source_sha256'].items():assert digest(ROOT/n)==sha,n
    assert (base/build['log']).is_file()
    phases={};totals={}
    for phase,n in [('correctness',1),('smoke',6),('comparison',48)]:
        data=read(base/('relation-pack-'+phase+'.json'));phases[phase]=data
        assert data['passed'] and data['phase']==phase and len(data['runs'])==n
        assert data['build_metadata']==build
        assert data['runner_sha256']==digest(snap/'relation_pack_sweep.py')
        assert data['execute_sha256']==digest(snap/'pack_run.py')
        for run in data['runs']:
            assert run['status']=='passed'
            raw=marked((base/run['log']).read_text(),'EVENT_LAB ');assert len(raw)==len(run['rows'])
            for parsed,row in zip(raw,run['rows']):
                assert all(row[k]==v for k,v in parsed.items())
                assert parsed.get('passed',parsed.get('verified',False))
        totals[phase]=sum(len(run['rows']) for run in data['runs'])
    rows=phases['correctness']['runs'][0]['rows']
    separator=[r for r in rows if r['kind']=='separator_verification'];assert len(separator)==1 and separator[0]['cases']==12288
    relational=[r for r in rows if r['kind']=='relation_pack_verification']
    assert len(relational)==6 and {r['candidate'] for r in relational}==set(CANDIDATES)
    assert all(r['native_runs']==1984 for r in relational)
    assert totals==dict(correctness=54,smoke=48,comparison=384)
    comparison=read(base/'relation-pack-analysis.json');expected=analyze(base/'relation-pack-comparison.json');expected['record']='relation-pack-analysis.json'
    assert comparison==expected
    assert comparison['checker_sha256']==digest(snap/'relation_pack_analysis.py')
    repeat=read(base/'relation-pack-repeat.json')
    assert repeat['passed'] and len(repeat['runs'])==8 and repeat['trials']==11
    assert repeat['candidates']==['dense','packed512'] and repeat['widths']==[5]
    assert repeat['layouts']==['face-major','bit-major'] and repeat['fanouts']==[2,8]
    assert repeat['build_metadata']==build
    assert repeat['runner_sha256']==digest(snap/'relation_pack_repeat.py')
    assert repeat['execute_sha256']==digest(snap/'pack_run.py')
    for run in repeat['runs']:
        assert run['status']=='passed';reverse=int(run['job']['EVENT_PACK_FANOUT']==2)
        assert run['job']['EVENT_PACK_REVERSE']==reverse
        raw=marked((base/run['log']).read_text(),'EVENT_LAB ');assert len(raw)==len(run['rows'])==8
        for parsed,row in zip(raw,run['rows']):assert all(row[k]==v for k,v in parsed.items())
        first='factorized' if reverse else 'complete';second='complete' if reverse else 'factorized'
        assert [r['schedule'] for r in raw]==[first,first,second,second]*2
    repeated=read(base/'relation-pack-repeat-analysis.json');expected=analyze(base/'relation-pack-repeat.json');expected['record']='relation-pack-repeat-analysis.json'
    assert repeated==expected
    def key(p):return tuple(p[k] for k in ['candidate','width','layout','fanout','program','memo'])
    main={key(p):p for p in comparison['comparisons']}
    for p in repeated['comparisons']:
        for field in ['checksum','represented_bindings','complete_emissions','factorized_emissions']:assert p[field]==main[key(p)][field]
    for dirname in ['query-separator-initial','relation-pack-proof']:
        folder=base/dirname
        for n,sha in read(folder/'manifest.json').items():assert digest(folder/n)==sha,n
    proof=read(base/'query-separator-lean.json')
    assert proof['passed'] and len(proof['theorems'])==6 and all(not a for a in proof['theorems'].values())
    assert digest(LAB/proof['source'])==proof['source_sha256']
    assert digest(base/'relation-pack-proof'/Path(proof['source']).name)==proof['source_sha256']
    assert digest(Path(proof['command'][0]))==proof['lean_binary_sha256']
    log=(base/proof['log']).read_text()
    for theorem in proof['theorems']:assert "'QuerySeparator."+theorem+"' does not depend on any axioms" in log
    initial=read(base/'query-separator-initial/query-separator-lean-initial.json')
    assert initial['passed'] and not initial['axiom_free'] and len(initial['theorems'])==6
    assert sum(bool(v) for v in initial['theorems'].values())==4
    reading=read(base/'relation-pack-reading.json')
    for n,sha in reading['files'].items():assert digest(ROOT/n)==sha,n
    return dict(passed=True,binary_sha256=build['binary_sha256'],phase_rows=totals,
        native_correctness_runs=sum(r['native_runs'] for r in relational),separator_cases=12288,
        matched_schedule_pairs=len(comparison['comparisons']),repeated_schedule_pairs=len(repeated['comparisons']),additional_axiom_free_reports=6,
        checker_sha256=digest(Path(__file__)),
        boundary='Two-branch native typed relation programs on checked legal products with shared environments. A conservative normalized-IR checker validates a proposed separator; it does not search for optimal partitions. Six separate denotational Lean reports, not Rust refinement.')
if __name__=='__main__':
    r=verify();(LAB/'results/relation-pack-audit.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
