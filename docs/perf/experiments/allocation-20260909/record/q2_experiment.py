"""Exact isolated dense-WordMap candidate identity, preserving Q1 and prior work."""
import hashlib
import shutil
import subprocess
from diagnostics import ROUND, SOURCE, digest, load
from q1_experiment import fingerprint as q1_identity, prior_identities

TREE = ROUND/'q2-source'
Q1 = load(ROUND/'q1-gates-2/STATE.json')
def git(*args):
    return subprocess.check_output(['git', '-C', TREE, *args])

def fingerprint():
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert q1_identity() == Q1['source_fingerprint']
    prior_identities()
    untracked = sorted(filter(None, git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')))
    changed = set(git('diff', 'HEAD', '--name-only').decode().splitlines()) | {n.decode() for n in untracked}
    assert all(n in Q1['source_files'] or n.startswith('crates/bumbledb/src/exec/wordmap/') for n in changed), changed
    for name, sha in Q1['source_files'].items():
        if '/exec/wordmap' not in name and not name.endswith('/sink/tests/demand_growth.rs'):
            assert digest(TREE/name) == sha, name
    h = hashlib.sha256(git('diff', '--binary', 'HEAD'))
    for name in untracked:
        h.update(name+b'\0'); h.update(bytes.fromhex(digest(TREE/name.decode())))
    return h.hexdigest()

def freeze(phase):
    files = set(git('diff', 'HEAD', '--name-only').decode().splitlines()) | set(git('ls-files', '--others', '--exclude-standard').decode().splitlines())
    phase.state['source_files'] = {n:digest(TREE/n) for n in sorted(files)}
    phase.save()
    phase.run('source-patch', ['git', '-C', TREE, 'diff', '--binary', 'HEAD'])
    for n in files:
        dest = phase.path/'source'/n
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(TREE/n, dest)
