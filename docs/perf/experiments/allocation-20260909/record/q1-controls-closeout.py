#!/usr/bin/env python3
"""Verify broader Q1 evidence and restore unmounted ordinary source identities."""
import shutil
import subprocess
from diagnostics import Phase, ROUND, SOURCE, digest, load
from q1_experiment import TREE, fingerprint, prior_identities
phase=Phase('q1-controls-closeout-1');shutil.copy2(__file__,phase.path/'closeout.py')
baseline=ROUND/'q1-control-baseline-source'
try:
    expected=load(ROUND/'q1-candidate-closeout-1/STATE.json')
    source=fingerprint();identities=prior_identities()
    assert source==expected['source_fingerprint'] and identities==expected['identities']
    assert subprocess.check_output(['git','-C',baseline,'rev-parse','HEAD']).decode().strip()==SOURCE
    assert not subprocess.check_output(['git','-C',baseline,'status','--porcelain'])
    for variant in ('baseline','candidate'):
        path=ROUND/f'q1-controls-{variant}-2';state=load(path/'STATE.json')
        assert state['status']=='BROADER-CONTROLS-COMPLETE-REVIEW-REQUIRED'
        assert all(digest(name)==sha for name,sha in state['dependencies'].items())
        assert digest(path/'controls-test')==state['binary_sha256']
    review=load(ROUND/'q1-controls-review-1/STATE.json')
    assert review['status']=='ALL-BROADER-OWNERS-AND-GROWTH-TRADEOFFS-VERIFIED'
    assert load(ROUND/'q1-controls-candidate-1/STATE.json')['status']=='INVALID-BASELINE-EXECUTABLE-REUSED'
    assert load(ROUND/'q1-controls-baseline-1/STATE.json')['status']=='SUPERSEDED-CONTROL-COVERAGE-GAP'
    phase.state.update(source_fingerprint=source,identities=identities,baseline_source=SOURCE,
        next='Matched ordinary timing including preparation and rollover tails; no repeated allocation panel/full trace/full benchmark/commit/push/release.')
    phase.save()
    for label,tree in [('candidate',TREE),('baseline',baseline)]:
        phase.run(label+'-diff',['git','-C',tree,'diff','--check'])
        phase.run(label+'-format',['cargo','fmt','--manifest-path',tree/'Cargo.toml','--all','--check'])
    assert fingerprint()==source and prior_identities()==identities
except BaseException:
    phase.finish('INCOMPLETE');raise
else:phase.finish('BROADER-EVIDENCE-AND-UNMOUNTED-SOURCES-VERIFIED')
