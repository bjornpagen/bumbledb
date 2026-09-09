"""Exact same-path source identities for ordinary Q1 timing builds."""
import hashlib
import subprocess
from diagnostics import ROUND,SOURCE,load,digest
from q1_experiment import fingerprint as candidate_fingerprint,prior_identities

TREE=ROUND/'q1-control-baseline-source'
MANIFEST='crates/bumbledb-bench/Cargo.toml'
HOOK=f'\n[[bin]]\nname = "q1-timing"\npath = "{ROUND}/q1_timing.rs"\n'
GATE=load(ROUND/'q1-gates-2/STATE.json')
def git(*args):return subprocess.check_output(['git','-C',TREE,*args])
def source(variant):
    assert git('rev-parse','HEAD').decode().strip()==SOURCE
    assert candidate_fingerprint()==GATE['source_fingerprint']
    prior_identities()
    expected=GATE['source_files'] if variant=='candidate' else {}
    changed=set(git('diff','HEAD','--name-only').decode().splitlines())|set(git('ls-files','--others','--exclude-standard').decode().splitlines())
    assert changed==set(expected)|{MANIFEST},changed
    for name in changed:
        actual=(TREE/name).read_bytes()
        if name==MANIFEST:
            assert actual==git('show','HEAD:'+name)+HOOK.encode()
        else:
            assert hashlib.sha256(actual).hexdigest()==expected[name],name
    return {name:digest(TREE/name) for name in sorted(changed)}

def dependencies():
    paths=[ROUND/'q1_timing.rs',ROUND/'q1_control_cases.rs']
    paths += [TREE/'crates/bumbledb-bench'/name for name in
        ['src/families/read.rs','src/translate/goldens.rs','src/schema.rs','src/boost.rs','src/clockproxy.rs','src/harness/work.rs']]
    return {str(path):digest(path) for path in paths}
