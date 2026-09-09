#!/usr/bin/env python3
"""Frozen baseline/candidate broader allocation controls; no timing/tracing."""
import argparse
import hashlib
import json
import shutil
import subprocess
from diagnostics import ENV, Phase, REPO, ROUND, SOURCE, digest, load
from q1_experiment import prior_identities

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant',choices=['baseline','candidate'])
parser.add_argument('attempt',type=int)
args=parser.parse_args()
tree=ROUND/('q1-source' if args.variant=='candidate' else 'q1-control-baseline-source')
gate=load(ROUND/'q1-gates-2/STATE.json')
assert gate['status']=='CHECKS-COMPLETE-REVIEW-REQUIRED'
mounts={
    'crates/bumbledb/src/api/prepared/tests.rs':('q1_controls.rs','mod q1_controls;'),
    'crates/bumbledb/src/exec/sink.rs':('q1_control_sink.rs','mod q1_control;'),
    'crates/bumbledb/src/exec/wordmap.rs':('q1_control_wordmap.rs','mod q1_control;'),
}
def git(*args):return subprocess.check_output(['git','-C',tree,*args])
def source():
    assert git('rev-parse','HEAD').decode().strip()==SOURCE
    expected=gate['source_files'] if args.variant=='candidate' else {}
    changed=set(git('diff','HEAD','--name-only').decode().splitlines())|set(git('ls-files','--others','--exclude-standard').decode().splitlines())
    assert changed==set(expected)|set(mounts),changed
    for name in changed:
        actual=(tree/name).read_bytes()
        if name in mounts:
            external,declaration=mounts[name]
            hook=(f'\n#[path = "{ROUND/external}"]\n{declaration}\n').encode()
            if not name.endswith('/tests.rs'):hook=b'\n#[cfg(test)]'+hook
            assert actual.count(hook)==1,name
            actual=actual.replace(hook,b'')
        if name in expected:assert hashlib.sha256(actual).hexdigest()==expected[name],name
        else:assert actual==git('show','HEAD:'+name),name
    return hashlib.sha256(git('diff','--binary','HEAD')).hexdigest()
phase=Phase(f'q1-controls-{args.variant}-{args.attempt}')
original=ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
dependencies=[ROUND/pair[0] for pair in mounts.values()]+[ROUND/'q1_control_cases.rs',ROUND/'q1-source/crates/bumbledb-bench/src/schema.rs']
hashes={str(path):digest(path) for path in dependencies}
inputs={name:digest(original/name) for name in ('db/data.mdb','oracle.sqlite')}
baseline=load(ROUND/'q1-prepare-1/STATE.json')
assert inputs['db/data.mdb']==baseline['database_sha256'] and inputs['oracle.sqlite']==baseline['oracle_sha256']
ENV.update(CARGO_BUILD_JOBS='1',CARGO_TARGET_DIR=str(phase.path/'cargo-target'),
           CARGO_BUILD_BUILD_DIR=str(phase.path/'cargo-build'))
try:
    mounted=source()
    phase.state.update(variant=args.variant,source_tree=str(tree),mounted_sha256=mounted,
        ordinary_source=gate['source_fingerprint'] if args.variant=='candidate' else SOURCE,
        dependencies=hashes,input_hashes=inputs,prior_identities=prior_identities(),
        protocol='41 case/draw controls, each prepare/cold/warm/small/return/release/refill/warm_refill; exact numeric handwritten SQLite goldens for every execution; counters stop before observation; existing saved corpus, no timings/full trace')
    phase.save()
    shutil.copy2(__file__,phase.path/'controls.py')
    for path in dependencies:shutil.copy2(path,phase.path/path.name)
    phase.run('source-patch',['git','-C',tree,'diff','--binary','HEAD'])
    shutil.copytree(original/'db',phase.path/'db')
    shutil.copy2(original/'oracle.sqlite',phase.path/'oracle.sqlite')
    phase.run('build',['cargo','test','--manifest-path',tree/'Cargo.toml','--locked','-p','bumbledb','--features','alloc-counter','--lib','--no-run','--message-format=json'])
    artifacts=[r for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (r:=json.loads(line)).get('reason')=='compiler-artifact'
        and r.get('executable') and r.get('target',{}).get('name')=='bumbledb']
    assert len(artifacts)==1 and not artifacts[0]['fresh']
    assert artifacts[0]['package_id'].startswith('path+file://'+str(tree)+'/crates/bumbledb#')
    assert artifacts[0]['executable'].startswith(str(phase.path)+'/')
    binary=phase.path/'controls-test';shutil.copy2(artifacts[0]['executable'],binary)
    phase.state['engine_artifact']=artifacts[0]
    phase.state['binary_sha256']=digest(binary);phase.save()
    phase.run('controls',['env','BUMBLEDB_Q1_DB='+str(phase.path/'db'),'BUMBLEDB_Q1_ORACLE='+str(phase.path/'oracle.sqlite'),binary,'q1_controls::saved_controls','--ignored','--nocapture','--test-threads=1'])
    raw=(phase.path/'controls.log').read_text()
    assert 'PASS controls cases=41 windows=328' in raw
    import re
    maps=re.findall(r'^MAP (\S+) role=(\S+) rows=(\d+) arity=(\d+) owners=(\[.*?\]) bytes=(\d+)$',raw,re.M)
    assert any(label=='union:100000:cold/main' and int(rows)==100000 and int(size)>0 for label,role,rows,arity,owners,size in maps),'union must actually exercise a hashed multi-rule result'
    if args.variant=='candidate':
        assert all(int(size)==0 for label,role,rows,arity,owners,size in maps if ':prepare/' in label),'compiled candidate must have empty unexecuted hash tables'
    assert source()==mounted and prior_identities()==phase.state['prior_identities']
    assert all(digest(path)==sha for path,sha in hashes.items())
    assert all(digest(original/name)==sha==digest(phase.path/name) for name,sha in inputs.items())
except BaseException:
    phase.finish('INCOMPLETE');raise
else:phase.finish('BROADER-CONTROLS-COMPLETE-REVIEW-REQUIRED')
