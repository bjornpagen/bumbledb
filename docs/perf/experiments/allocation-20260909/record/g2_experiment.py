"""Source identity and freezing for isolated G2 staged construction."""
import hashlib
import shutil
import subprocess
from diagnostics import ROUND, SOURCE, digest

TREE = ROUND/'g2-source'
ALLOWED = {
    'crates/bumbledb/src/exec/colt/grow.rs',
    'crates/bumbledb/src/exec/colt/tests.rs',
    'crates/bumbledb/src/exec/colt/tests/admit.rs',
    'crates/bumbledb/src/exec/colt/tests/construction_tails.rs',
    'crates/bumbledb/src/exec/colt/tests/staged_growth.rs',
}

def git(*args):
    return subprocess.check_output(['git', '-C', str(TREE), *args])

def fingerprint():
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    names = list(filter(None, git('ls-files','--others','--exclude-standard','-z').split(b'\0')))
    changed = set(git('diff','HEAD','--name-only').decode().splitlines()) | {n.decode() for n in names}
    assert changed <= ALLOWED, changed-ALLOWED
    result = hashlib.sha256(git('diff','--binary','HEAD'))
    for name in sorted(names):
        result.update(name+b'\0')
        result.update(bytes.fromhex(digest(TREE/name.decode())))
    return result.hexdigest()

def freeze(phase):
    phase.run('source-patch',['git','-C',TREE,'diff','--binary','HEAD'])
    shutil.copy2(__file__,phase.path/'g2_experiment.py')
    for name in filter(None,git('ls-files','--others','--exclude-standard','-z').split(b'\0')):
        dest = phase.path/'untracked'/name.decode()
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(TREE/name.decode(),dest)
