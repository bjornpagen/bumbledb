"""Single-source dense publication on frozen Q2; other candidates untouched."""
import hashlib
import shutil
import subprocess
from diagnostics import ROUND, SOURCE, digest, load
from q2_experiment import fingerprint as q2_identity, prior_identities

TREE = ROUND/'q3-source'
Q2 = load(ROUND/'q2-gates-2/STATE.json')

def git(*args):
    return subprocess.check_output(['git', '-C', TREE, *args])

def fingerprint():
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert q2_identity() == Q2['source_fingerprint']
    prior_identities()
    untracked = sorted(filter(None, git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')))
    changed = set(git('diff', 'HEAD', '--name-only').decode().splitlines()) | {n.decode() for n in untracked}
    assert changed == set(Q2['source_files']), changed
    for name, sha in Q2['source_files'].items():
        if '/exec/wordmap' not in name:
            assert digest(TREE/name) == sha, name
    h = hashlib.sha256(git('diff', '--binary', 'HEAD'))
    for name in untracked:
        h.update(name+b'\0')
        h.update(bytes.fromhex(digest(TREE/name.decode())))
    return h.hexdigest()

def freeze(phase):
    files = set(git('diff', 'HEAD', '--name-only').decode().splitlines()) | set(git('ls-files', '--others', '--exclude-standard').decode().splitlines())
    phase.state['source_files'] = {n:digest(TREE/n) for n in sorted(files)}
    phase.save()
    phase.run('source-patch', ['git', '-C', TREE, 'diff', '--binary', 'HEAD'])
    for name in files:
        dest = phase.path/'source'/name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(TREE/name, dest)
