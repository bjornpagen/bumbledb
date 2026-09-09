"""Source identity and freezing for isolated G1 construction tails."""
import hashlib
import shutil
import subprocess
from diagnostics import ROUND, SOURCE, digest

TREE = ROUND/'g1-source'
ALLOWED = {
    'crates/bumbledb/src/exec/colt/force.rs',
    'crates/bumbledb/src/exec/colt/grow.rs',
    'crates/bumbledb/src/exec/colt/tests.rs',
    'crates/bumbledb/src/exec/colt/tests/construction_tails.rs',
}
def git(*args):
    return subprocess.check_output(['git', '-C', str(TREE), *args])

def fingerprint(observer=False, timing=False):
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    names = list(filter(None, git('ls-files','--others','--exclude-standard','-z').split(b'\0')))
    changed = set(git('diff','HEAD','--name-only').decode().splitlines()) | {n.decode() for n in names}
    allowed = ALLOWED | ({'crates/bumbledb/src/exec/colt.rs',
        'crates/bumbledb/src/api/prepared/tests.rs'} if observer else set())
    if timing:
        allowed.add('crates/bumbledb-bench/Cargo.toml')
    assert changed <= allowed, changed-allowed
    result = hashlib.sha256(git('diff','--binary','HEAD'))
    for name in sorted(names):
        result.update(name+b'\0')
        result.update(bytes.fromhex(digest(TREE/name.decode())))
    return result.hexdigest()

def freeze(phase):
    phase.run('source-patch',['git','-C',TREE,'diff','--binary','HEAD'])
    shutil.copy2(__file__,phase.path/'g1_experiment.py')
    for name in filter(None,git('ls-files','--others','--exclude-standard','-z').split(b'\0')):
        dest = phase.path/'untracked'/name.decode()
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(TREE/name.decode(),dest)
