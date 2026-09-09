"""Exact mounted Q1/Q2 owner observers in one disposable comparison checkout."""
import hashlib
import subprocess
from diagnostics import ROUND, SOURCE, digest, load
from q2_experiment import fingerprint, prior_identities

TREE = ROUND/'q1-control-baseline-source'
GATES = {v: load(ROUND/f'{v}-gates-2/STATE.json') for v in ['q1', 'q2']}

def git(*args):
    return subprocess.check_output(['git', '-C', TREE, *args])

def hooks(variant):
    def hook(external, declaration, cfg=False):
        return ('\n#[cfg(test)]' if cfg else '') + f'\n#[path = "{ROUND/external}"]\n{declaration}\n'
    return {
        'crates/bumbledb/src/api/prepared/tests.rs': hook('q1_controls.rs', 'mod q1_controls;'),
        'crates/bumbledb/src/exec/sink.rs': hook('q1_control_sink.rs', 'mod q1_control;', True),
        'crates/bumbledb/src/exec/wordmap.rs':
            hook(f'{variant}_control_wordmap.rs', 'mod q1_control;', True)
            + hook('q2_payload_regression.rs', 'mod q2_payload_regression;', True),
    }

def dependencies(variant):
    names = ['q1_controls.rs', 'q1_control_cases.rs', 'q1_control_sink.rs',
             f'{variant}_control_wordmap.rs', 'q2_payload_regression.rs',
             'q1-source/crates/bumbledb-bench/src/schema.rs']
    return {str(ROUND/n): digest(ROUND/n) for n in names}

def source(variant):
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert fingerprint() == GATES['q2']['source_fingerprint']
    expected = GATES[variant]['source_files']
    mounted = hooks(variant)
    changed = set(git('diff', 'HEAD', '--name-only').decode().splitlines()) | set(git('ls-files', '--others', '--exclude-standard').decode().splitlines())
    assert changed == set(expected) | set(mounted), changed
    for name in changed:
        actual = (TREE/name).read_bytes()
        if name in mounted:
            hook = mounted[name].encode()
            assert actual.endswith(hook) and actual.count(hook) == 1, name
            actual = actual[:-len(hook)]
        if name in expected:
            assert hashlib.sha256(actual).hexdigest() == expected[name], name
        else:
            assert actual == git('show', 'HEAD:'+name), name
    return {name: digest(TREE/name) for name in sorted(changed)}
