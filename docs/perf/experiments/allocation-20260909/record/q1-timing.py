#!/usr/bin/env python3
"""One fixed ordinary Q1 ABBA panel; no retries, profile capture or full suite."""
import argparse
import json
import shutil
import subprocess
from diagnostics import Phase,ROUND,SOURCE,digest,load
from q1_experiment import fingerprint,prior_identities
from q1_time_experiment import TREE,dependencies

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt',type=int)
parser.add_argument('--baseline-build',type=int,default=2)
parser.add_argument('--candidate-build',type=int,default=1)
args=parser.parse_args()
builds={'A':ROUND/f'q1-time-build-baseline-{args.baseline_build}','B':ROUND/f'q1-time-build-candidate-{args.candidate_build}'}
states={key:load(path/'STATE.json') for key,path in builds.items()}
for key,state in states.items():
    assert state['status']=='ORDINARY-TIMING-BINARY-FROZEN' and state['variant']=={'A':'baseline','B':'candidate'}[key]
    assert state['source_tree']==str(TREE) and not state['engine_artifact']['fresh']
    assert state['engine_artifact']['features']==['collision-probe']
    assert all(step['status']=='PASS' for step in state['steps'])
assert states['A']['binary_sha256']!=states['B']['binary_sha256']
assert states['A']['dependencies']==states['B']['dependencies']==dependencies()
assert not subprocess.check_output(['git','-C',TREE,'status','--porcelain']), 'Restore comparison source before timing'
assert subprocess.check_output(['git','-C',TREE,'rev-parse','HEAD']).decode().strip()==SOURCE
identity=fingerprint();assert identity==states['B']['candidate_identity']
identities=prior_identities()
# Fixed before observing timings: saved main cases plus positive and adverse
# allocation controls, not only the low-result/high-hint winners.
cases=[(family,draw) for draw in range(4) for family in ['saved_triangle','saved_point','saved_range']]
cases += [('computed',i) for i in [0,1,4]]+[('union',i) for i in [2,4]]+[('groups',i) for i in [1,3,4]]
cases += [('dnf_groups',0),('dnf_groups',2),('pairs',2),('dense',2),('interior',4),('reach',2),('interior_reach',2),('union_groups',2)]
assert len(cases)==len(set(cases))==28
order=['A0','B0','B1','A1']
original=ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
hashes={name:digest(original/name) for name in ['db/data.mdb','oracle.sqlite']}
phase=Phase(f'q1-timing-{args.attempt}')
shutil.copy2(__file__,phase.path/'timing.py')
for key,path in builds.items():shutil.copy2(path/'STATE.json',phase.path/f'{key}-build-STATE.json')
phase.state.update(diagnostic_only=False,source_fingerprint=identity,prior_identities=identities,cases=cases,order=order,
    executables={key:dict(path=str(path/'q1-timing'),sha256=states[key]['binary_sha256']) for key,path in builds.items()},
    dependencies=dependencies(),input_hashes=hashes,
    protocol='Per-case ABBA, no retries/filtering/clock normalization. Cold: 2 discarded then 16 fresh DB reopen+prepare calls; DB open and AST/bind-array/empty-answer construction excluded. Separate prepare, first and jointly measured prepare+first, then second. Both warm modes: independent prepare, 8 warmups + 320 individual calls; alternating executes/checks the next fixture draw outside timer before each target. SQL expected values computed before timings, exact outputs checked after every operation. WorkContext creation remains inside ordinary prepare/read as in existing benchmarks.',
    rollover_probe=dict(same_target_index=248,basis='8 warmups; single reset per call, first empty reset does not advance generation. Source-model prediction for nonempty single-sink saved triangle, not automatic attribution of all tails.'),
    limitations='Shared macOS host; verified QoS steering, not hard P-core affinity. One non-retrying clock bracket around the cold pair panel, one for each warm mode. Result checker outside timing uses one numeric row buffer and sorting; it can warm/evict caches. Fresh DB reopen is not an OS-cold-cache claim. No allocator/CPU instrumentation/full suite/RSS/Pi qualification.')
phase.save();reports={}
try:
    shutil.copytree(original/'db',phase.path/'db');shutil.copy2(original/'oracle.sqlite',phase.path/'oracle.sqlite')
    for family,draw in cases:
        for label in order:
            key=label[0];binary=builds[key]/'q1-timing'
            assert digest(binary)==states[key]['binary_sha256']
            assert fingerprint()==identity and prior_identities()==identities
            name=f'{family}-{draw}-{label}'
            phase.run(name,[binary,phase.path/'db',phase.path/'oracle.sqlite',family,str(draw)])
            raw=(phase.path/f'{name}.log').read_text()
            assert 'scheduler boost: user-interactive QoS verified (not hard P-core affinity)' in raw
            rows=[json.loads(line) for line in raw.splitlines() if line.startswith('{')]
            assert len(rows)==7
            meta,*blocks=rows
            assert meta['family']==family and meta['draw']==draw and meta['alternate']==(draw+1)%meta['sql_draws']
            assert meta['warmups']==8 and meta['warm_samples']==320
            assert [b['kind'] for b in blocks]==['prepare','first','combined','second','same_target','alternating']
            if family=='saved_triangle':
                assert meta['parameters']==[[1,6],[167,172],[333,338],[500,500]][draw]
                assert meta['answers']==(0 if draw==3 else 5)
            if family=='saved_point':assert meta['answers']==(0 if draw==3 else 1)
            if family=='saved_range':assert meta['answers']==2000
            for block in blocks:
                assert len(block['raw_ns'])==(16 if block['kind'] in ['prepare','first','combined','second'] else 320)
                assert block['batch']==1 and not block['ghz']['retried']
            assert all(t>=p+f for p,f,t in zip(blocks[0]['raw_ns'],blocks[1]['raw_ns'],blocks[2]['raw_ns']))
            reports[name]=rows
            for input_name,sha in hashes.items():assert digest(phase.path/input_name)==sha
    assert dependencies()==states['A']['dependencies']
    assert all(digest(original/name)==sha for name,sha in hashes.items())
    assert fingerprint()==identity and prior_identities()==identities
    assert not subprocess.check_output(['git','-C',TREE,'status','--porcelain'])
    phase.state.update(processes_completed=len(reports),distributions=6*len(reports));phase.save()
except BaseException:
    phase.finish('INCOMPLETE');raise
else:phase.finish('ORDINARY-Q1-TIMING-COMPLETE-REVIEW-REQUIRED')
