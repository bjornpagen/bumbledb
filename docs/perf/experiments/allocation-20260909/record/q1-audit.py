#!/usr/bin/env python3
"""Matched saved owners and preparation counters for the gated Q1 candidate."""
import argparse
import hashlib
import json
import shutil
import subprocess
from diagnostics import ENV, Phase, REPO, ROUND, SOURCE, digest, load
from q1_experiment import TREE, git, prior_identities

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('gate',type=int)
parser.add_argument('attempt',type=int)
args=parser.parse_args()
gate_dir=ROUND/f'q1-gates-{args.gate}'
gate=load(gate_dir/'STATE.json')
assert gate['status']=='CHECKS-COMPLETE-REVIEW-REQUIRED'
phase=Phase(f'q1-audit-{args.attempt}')
mounts={
    'crates/bumbledb/src/api/prepared/tests.rs':('q1_prepare_candidate.rs','mod q1_saved;'),
    'crates/bumbledb/src/exec/colt.rs':('q1_colt_observer.rs','mod q1_observer;'),
    'crates/bumbledb/src/exec/sink.rs':('q1_sink_observer.rs','mod q1_observer;'),
    'crates/bumbledb/src/exec/wordmap.rs':('q1_wordmap_observer.rs','mod q1_observer;'),
    'crates/bumbledb/src/image/view.rs':('q1_view_observer.rs','pub(crate) mod q1_observer;'),
}

def source():
    assert git('rev-parse','HEAD').decode().strip()==SOURCE
    changed=set(git('diff','HEAD','--name-only').decode().splitlines())
    untracked=set(git('ls-files','--others','--exclude-standard').decode().splitlines())
    assert changed|untracked == set(gate['source_files'])|set(mounts)
    for name in changed|untracked:
        actual=(TREE/name).read_bytes()
        if name in mounts:
            external,declaration=mounts[name]
            hook=(f'\n#[path = "{ROUND/external}"]\n{declaration}\n').encode()
            if not name.endswith('/tests.rs'):
                hook=b'\n#[cfg(test)]'+hook
            assert actual.count(hook)==1,name
            actual=actual.replace(hook,b'')
        if name in gate['source_files']:
            assert hashlib.sha256(actual).hexdigest()==gate['source_files'][name],name
        else:
            assert actual==git('show','HEAD:'+name),name
    return hashlib.sha256(git('diff','--binary','HEAD')).hexdigest()

dependencies=[ROUND/pair[0] for pair in mounts.values()]+[ROUND/'q1_saved.rs',TREE/'crates/bumbledb-bench/src/schema.rs']
hashes={str(path):digest(path) for path in dependencies}
baseline=load(ROUND/'q1-prepare-1/STATE.json')
# Only omit two post-snapshot estimate-printing lines: execution no longer
# carries planner estimates. Keep the original frozen helper intact.
old_prepare=ROUND/'q1_prepare.rs'
new_prepare=ROUND/'q1_prepare_candidate.rs'
assert digest(old_prepare)==baseline['dependencies'][str(old_prepare)]
omitted=(
    '        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else { panic!("Free Join expected"); };\n'
    '        println!("ESTIMATES {family} {:?}", rule.plan.estimates());\n'
)
assert old_prepare.read_text().count(omitted)==1
assert new_prepare.read_text()==old_prepare.read_text().replace(omitted,'')
assert {name:sha for name,sha in hashes.items() if name!=str(new_prepare)}=={
    name:sha for name,sha in baseline['dependencies'].items() if name!=str(old_prepare)}
original=ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
db_hash=digest(original/'db/data.mdb')
oracle_hash=digest(original/'oracle.sqlite')
assert db_hash==baseline['database_sha256'] and oracle_hash==baseline['oracle_sha256']
ENV.update(CARGO_BUILD_JOBS='1',CARGO_TARGET_DIR=str(REPO/'target'))
try:
    source_hash=source()
    phase.state.update(source_fingerprint=gate['source_fingerprint'],gate=str(gate_dir),
        source_tree=str(TREE),hooks_sha256=source_hash,dependencies=hashes,
        prior_identities=prior_identities(),database_sha256=db_hash,oracle_sha256=oracle_hash,
        protocol='Frozen Q1 queries/owners; candidate preparation helper omits only two post-snapshot estimate-printing lines; 32 saved SQL execution windows and separate preparation-only windows; no timings',
        preparation_observation_delta=omitted)
    phase.save()
    shutil.copy2(__file__,phase.path/'audit.py')
    for path in dependencies:
        shutil.copy2(path,phase.path/path.name)
    phase.run('source-patch',['git','-C',TREE,'diff','--binary','HEAD'])
    shutil.copytree(original/'db',phase.path/'db')
    shutil.copy2(ROUND/'q1-owners-1/oracle.log',phase.path/'oracle.log')
    assert digest(phase.path/'db/data.mdb')==db_hash
    phase.run('build',['cargo','test','--manifest-path',TREE/'Cargo.toml','--locked','-p','bumbledb',
        '--features','alloc-counter','--lib','--no-run','--message-format=json'])
    binaries=[r['executable'] for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (r:=json.loads(line)).get('reason')=='compiler-artifact'
        and r.get('executable') and r.get('target',{}).get('name')=='bumbledb']
    assert len(binaries)==1
    binary=phase.path/'saved-test'
    shutil.copy2(binaries[0],binary)
    phase.state['binary_sha256']=digest(binary)
    phase.save()
    for label,test,expected in (
        ('ownership','saved_output_and_survivors','PASS saved output and survivor ownership; 32 exact SQL windows'),
        ('preparation','saved_preparation','PASS saved preparation; two queries before any execution')):
        phase.run(label,['env','BUMBLEDB_Q1_DB='+str(phase.path/'db'),'BUMBLEDB_Q1_ORACLE='+str(phase.path/'oracle.log'),
            binary,'q1_saved::'+test,'--ignored','--nocapture','--test-threads=1'])
        assert expected in (phase.path/f'{label}.log').read_text()
    assert source()==source_hash
    assert all(digest(path)==sha for path,sha in hashes.items())
    assert prior_identities()==phase.state['prior_identities']
    assert digest(original/'db/data.mdb')==db_hash==digest(phase.path/'db/data.mdb')
    assert digest(original/'oracle.sqlite')==oracle_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('PREPARATION-AND-OWNERS-COMPLETE-REVIEW-REQUIRED')
