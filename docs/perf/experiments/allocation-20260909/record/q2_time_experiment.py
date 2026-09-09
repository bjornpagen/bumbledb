"""Same-path ordinary Q2-vs-Q1 experiment; no mounted allocation observers."""
import subprocess
from diagnostics import ROUND, SOURCE, digest, load
from q2_experiment import fingerprint, prior_identities

TREE = ROUND/'q1-control-baseline-source'
MANIFEST = 'crates/bumbledb-bench/Cargo.toml'
HOOK = f'\n[[bin]]\nname = "q1-timing"\npath = "{ROUND}/q1_timing.rs"\n'
GATES = {v: load(ROUND/f'{v}-gates-2/STATE.json') for v in ['q1', 'q2']}

# Predeclared before Q2 allocation results or ordinary timings. Price the
# reported Q1 losses, tiny/empty ZST seen maps, and unchanged-path controls.
CASES = [('dnf_groups', 2), ('union', 4), ('groups', 3), ('union_groups', 2),
         ('computed', 4), ('groups', 4), ('pairs', 2),
         ('computed', 0), ('computed', 1), ('groups', 1), ('reach', 2),
         ('saved_triangle', 0), ('saved_triangle', 3),
         ('saved_range', 0), ('saved_range', 2), ('saved_point', 0),
         ('saved_point', 3), ('dense', 2)]

def git(*args):
    return subprocess.check_output(['git', '-C', TREE, *args])

def source(variant):
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert fingerprint() == GATES['q2']['source_fingerprint']
    expected = GATES[variant]['source_files']
    changed = set(git('diff', 'HEAD', '--name-only').decode().splitlines()) | set(git('ls-files', '--others', '--exclude-standard').decode().splitlines())
    assert changed == set(expected) | {MANIFEST}, changed
    for name in changed:
        if name == MANIFEST:
            assert (TREE/name).read_bytes() == git('show', 'HEAD:'+name) + HOOK.encode()
        else:
            assert digest(TREE/name) == expected[name], name
    return {name: digest(TREE/name) for name in sorted(changed)}

def dependencies():
    paths = [ROUND/'q1_timing.rs', ROUND/'q1_control_cases.rs', ROUND/'q2_time_experiment.py']
    paths += [TREE/'crates/bumbledb-bench'/name for name in
        ['src/families/read.rs', 'src/translate/goldens.rs', 'src/schema.rs',
         'src/boost.rs', 'src/clockproxy.rs', 'src/harness/work.rs']]
    return {str(p): digest(p) for p in paths}
