#!/usr/bin/env python3
"""Verify baseline restored, every prior candidate preserved, and frozen observers."""
import runpy
import shutil
import subprocess
from diagnostics import Phase, ROUND, SOURCE, digest, load
from g2_experiment import fingerprint as g2
from g1_experiment import fingerprint as g1
from e1_experiment import fingerprint as e1
from m1_experiment import fingerprint as m1
from overlap_experiment import fingerprint as p3

phase = Phase('q1-closeout-1')
shutil.copy2(__file__, phase.path/'closeout.py')
tree = ROUND/'q1-source'
expected = load(ROUND/'g2-closeout-3/STATE.json')['identities']
def identities():
    return {'E1':e1(), 'M1':m1(), 'P3':p3(), 'G1':g1(), 'G2':g2(),
            'P2':runpy.run_path(str(ROUND/'p2-gate.py'))['fingerprint']()}
def baseline():
    assert subprocess.check_output(['git','-C',tree,'rev-parse','HEAD']).decode().strip() == SOURCE
    assert not subprocess.check_output(['git','-C',tree,'status','--porcelain'])
try:
    baseline()
    phase.state['identities'] = identities()
    assert phase.state['identities'] == expected
    for name in ('q1-owners-1','q1-prepare-1'):
        state = load(ROUND/name/'STATE.json')
        assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
        assert all(digest(path) == sha for path, sha in state['dependencies'].items())
    assert load(ROUND/'q1-owners-review-1/STATE.json')['status'] == 'ALL-OWNERS-AND-PRIOR-TRIANGLE-COUNTS-VERIFIED'
    phase.save()
    phase.run('diff-check',['git','-C',tree,'diff','--check'])
    phase.run('format',['cargo','fmt','--manifest-path',tree/'Cargo.toml','--all','--check'])
    baseline()
    assert identities() == expected
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('BASELINE-RESTORED-PRIOR-CANDIDATES-AND-OBSERVERS-VERIFIED')
