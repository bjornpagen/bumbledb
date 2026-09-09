"""Source identity for the independent, detached P3 worktree."""
import hashlib
import subprocess
from diagnostics import ROUND, SOURCE, digest

TREE = ROUND / 'p3-source'
ALLOWED = {
    'crates/bumbledb/src/interval/overlap.rs',
    'crates/bumbledb/src/interval/overlap/boundaries.rs',
    'crates/bumbledb/src/exec/run/tests/intervals.rs',
    'crates/bumbledb/src/exec/run/tests/intervals/constraints.rs',
}

def git(*args):
    return subprocess.check_output(['git', '-C', str(TREE), *args])

def fingerprint(observer=False, demand=False):
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    untracked = list(filter(None, git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')))
    changed = set(git('diff', 'HEAD', '--name-only').decode().splitlines())
    changed.update(name.decode() for name in untracked)
    allowed = ALLOWED | ({'crates/bumbledb/src/api/prepared/tests.rs',
                          'crates/bumbledb/src/exec/run.rs'} if observer else set())
    if demand:
        allowed |= {'crates/bumbledb/src/api/prepared/tests.rs',
                    'crates/bumbledb/src/exec/run.rs',
                    'crates/bumbledb/src/exec/run/overlap_leaf.rs'}
    assert changed <= allowed, changed - allowed
    patch = git('diff', '--binary', 'HEAD')
    result = hashlib.sha256(patch)
    for name in sorted(untracked):
        result.update(name + b'\0')
        result.update(bytes.fromhex(digest(TREE / name.decode())))
    return result.hexdigest()

def assert_baseline_production():
    for name, hook in [
        ('crates/bumbledb/src/interval/overlap.rs', '\n#[cfg(test)]\nmod boundaries;\n'),
        ('crates/bumbledb/src/exec/run/tests/intervals.rs', '\nmod constraints;\n'),
    ]:
        current = (TREE / name).read_text()
        assert current.count(hook) == 1
        assert current.replace(hook, '') == git('show', 'HEAD:'+name).decode(), name
