"""Audit frozen native Pack schedules, differential cases and denotational proofs."""
from pathlib import Path
import itertools, json
from prepare import LAB, ROOT
from unary_audit import read, digest, marked
from pack_native_analysis import analyze, CANDIDATES, SCENARIOS

def verify():
    base=LAB/'results';build=read(base/'build-pack-native.json');snap=base/'pack-native-src'
    for name,sha in read(snap/'manifest.json').items():
        assert digest(snap/name)==sha,name
    assert digest(snap/'bumbledb-pack')==build['binary_sha256']
    for name,sha in build['source_sha256'].items():
        assert digest(snap/Path(name).name)==sha,name
        assert digest(LAB/name)==sha,name
    assert digest(snap/'pack_native_build.py')==build['builder_sha256']
    for name,sha in build['production_manifest']['source_sha256'].items():
        assert digest(ROOT/name)==sha,name
    assert (base/build['log']).is_file()
    phases={};totals={}
    for phase,file,processes in [('correctness','pack-native-correctness.json',1),
                                ('smoke','pack-native-smoke.json',6),
                                ('comparison','pack-native-comparison-raw.json',36)]:
        data=read(base/file);phases[phase]=data
        assert data['passed'] and data['phase']==phase and len(data['runs'])==processes
        assert data['build_metadata']==build
        assert data['runner_sha256']==digest(snap/'pack_native_sweep.py')
        assert data['execute_sha256']==digest(snap/'pack_run.py')
        for run in data['runs']:
            assert run['status']=='passed';log=(base/run['log']).read_text()
            raw=marked(log,'EVENT_LAB ');assert len(raw)==len(run['rows'])
            for parsed,row in zip(raw,run['rows']):
                assert all(row[k]==v for k,v in parsed.items())
                assert parsed.get('verified',parsed.get('passed',False))
        totals[phase]=sum(len(run['rows']) for run in data['runs'])
    checks=[r for run in phases['correctness']['runs'] for r in run['rows'] if r['kind']=='factorized_pack_verification']
    assert len(checks)==6 and {r['candidate'] for r in checks}==set(CANDIDATES)
    assert all(r['native_runs']==896 for r in checks)
    assert totals==dict(correctness=47,smoke=24,comparison=144)
    sweep=phases['comparison'];assert sweep['trials']==7 and sweep['fanouts']==[2,4,8]
    actual={(r['job']['EVENT_LAB_CANDIDATE'],r['job']['EVENT_LAB_SCENARIO'],r['job']['EVENT_PACK_FANOUT']) for r in sweep['runs']}
    assert actual==set(itertools.product(CANDIDATES,SCENARIOS,[2,4,8]))
    analysis=read(base/'pack-native-analysis.json')
    expected=analyze(base/'pack-native-comparison-raw.json');expected['record']='pack-native-analysis.json'
    assert analysis==expected
    assert analysis['checker_sha256']==digest(snap/'pack_native_analysis.py')
    crossover=read(base/'pack-native-crossover-raw.json')
    assert crossover['passed'] and len(crossover['runs'])==12 and crossover['trials']==11
    assert crossover['candidates']==['dense','packed512'] and crossover['fanouts']==[1,2,8]
    assert crossover['build_metadata']==build
    assert crossover['runner_sha256']==digest(snap/'pack_native_crossover.py')
    assert crossover['execute_sha256']==digest(snap/'pack_run.py')
    for run in crossover['runs']:
        assert run['status']=='passed' and run['job']['EVENT_PACK_REVERSE']==1
        raw=marked((base/run['log']).read_text(),'EVENT_LAB ')
        assert len(raw)==len(run['rows'])==4
        for parsed,row in zip(raw,run['rows']):assert all(row[k]==v for k,v in parsed.items())
        assert [r['schedule'] for r in raw]==['factorized','factorized','complete','complete']
    repeat=read(base/'pack-native-crossover-analysis.json')
    expected=analyze(base/'pack-native-crossover-raw.json');expected['record']='pack-native-crossover-analysis.json'
    assert repeat==expected
    proof=base/'pack-native-proof'
    for name,sha in read(proof/'manifest.json').items():
        assert digest(proof/name)==sha,name
    report_count=0
    for name,namespace,count in [('factorized-pack','FactorizedPack',6),
                                 ('participating-validation','ParticipatingValidation',2),
                                 ('relation-pack','RelationPack',4)]:
        record=read(base/(name+'-lean.json'))
        assert record['passed'] and len(record['theorems'])==count
        assert all(not axioms for axioms in record['theorems'].values())
        assert digest(LAB/record['source'])==record['source_sha256']
        assert digest(proof/Path(record['source']).name)==record['source_sha256']
        assert digest(proof/(name+'-lean.json'))==digest(base/(name+'-lean.json'))
        log=(base/record['log']).read_text()
        for theorem in record['theorems']:
            assert "'"+namespace+'.'+theorem+"' does not depend on any axioms" in log
        assert digest(Path(record['command'][0]))==record['lean_binary_sha256']
        report_count+=count
    reading=read(base/'pack-native-reading.json')
    for name,sha in reading['files'].items():assert digest(ROOT/name)==sha,name
    return dict(passed=True,binary_sha256=build['binary_sha256'],phase_rows=totals,
        native_correctness_runs=sum(r['native_runs'] for r in checks),matched_schedule_pairs=len(analysis['comparisons']),
        crossover_schedule_pairs=len(repeat['comparisons']),additional_axiom_free_reports=report_count,checker_sha256=digest(Path(__file__)),
        boundary='Constructed Cartesian clover; native branch scans and summary join. Participating validation has unordered fault-set semantics. Twelve separate Lean denotational reports, not Rust verification. Central 195-report suite unchanged.')

if __name__=='__main__':
    result=verify();(LAB/'results/pack-native-audit.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
