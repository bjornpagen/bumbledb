"""Source identities for isolated removal of speculative result-table hints."""
import hashlib
import runpy
import shutil
import subprocess
from diagnostics import ROUND, SOURCE, digest, load

TREE = ROUND/'q1-source'
ALLOWED = {
    'crates/bumbledb/src/plan/fj.rs',
    'crates/bumbledb/src/plan/fj/derive_nodes.rs',
    'crates/bumbledb/src/api/prepared/build.rs',
    'crates/bumbledb/src/api/prepared/tests/correctness.rs',
    'crates/bumbledb/src/api/prepared/tests/projection_unique.rs',
    'crates/bumbledb/src/api/prepared/reach/spill_bounded.rs',
    'crates/bumbledb/src/exec/sink.rs',
    'crates/bumbledb/src/exec/sink/projection/new.rs',
    'crates/bumbledb/src/exec/sink/aggregate/new.rs',
    'crates/bumbledb/src/exec/wordmap.rs',
    'crates/bumbledb/src/exec/wordmap/new.rs',
    'crates/bumbledb/src/exec/sink/tests.rs',
    'crates/bumbledb/src/exec/sink/tests/demand_growth.rs',
}

def git(*args):
    return subprocess.check_output(['git','-C',str(TREE),*args])

def fingerprint():
    assert git('rev-parse','HEAD').decode().strip() == SOURCE
    untracked = list(filter(None,git('ls-files','--others','--exclude-standard','-z').split(b'\0')))
    changed = set(git('diff','HEAD','--name-only').decode().splitlines()) | {name.decode() for name in untracked}
    assert all(name in ALLOWED or name.startswith('crates/bumbledb/src/exec/sink/tests/') for name in changed), changed-ALLOWED
    result = hashlib.sha256(git('diff','--binary','HEAD'))
    for name in sorted(untracked):
        result.update(name+b'\0')
        result.update(bytes.fromhex(digest(TREE/name.decode())))
    return result.hexdigest()

def prior_identities():
    from g2_experiment import fingerprint as g2
    from g1_experiment import fingerprint as g1
    from e1_experiment import fingerprint as e1
    from m1_experiment import fingerprint as m1
    from overlap_experiment import fingerprint as p3
    actual = {'E1':e1(), 'M1':m1(), 'P3':p3(), 'G1':g1(), 'G2':g2(),
              'P2':runpy.run_path(str(ROUND/'p2-gate.py'))['fingerprint']()}
    assert actual == load(ROUND/'q1-closeout-1/STATE.json')['identities']
    return actual

def freeze(phase):
    phase.run('source-patch',['git','-C',TREE,'diff','--binary','HEAD'])
    shutil.copy2(__file__,phase.path/'q1_experiment.py')
    untracked = list(filter(None,git('ls-files','--others','--exclude-standard','-z').split(b'\0')))
    changed = git('diff','HEAD','--name-only').decode().splitlines() + [name.decode() for name in untracked]
    phase.state['source_files'] = {name: digest(TREE/name) for name in changed}
    phase.save()
    for name in changed:
        dest = phase.path/'source'/name
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(TREE/name,dest)
    for name in untracked:
        dest = phase.path/'untracked'/name.decode()
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(TREE/name.decode(),dest)
