#!/usr/bin/env python3
"""Verify exact unmounted Q1 source and preserve all prior candidates/evidence."""
import argparse
import shutil
from diagnostics import Phase, ROUND, digest, load
from q1_experiment import TREE, fingerprint, prior_identities

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('gate',type=int)
parser.add_argument('audit',type=int)
parser.add_argument('review',type=int)
parser.add_argument('attempt',type=int)
args=parser.parse_args()
gate=load(ROUND/f'q1-gates-{args.gate}/STATE.json')
audit=load(ROUND/f'q1-audit-{args.audit}/STATE.json')
review=load(ROUND/f'q1-audit-review-{args.review}/STATE.json')
phase=Phase(f'q1-candidate-closeout-{args.attempt}')
shutil.copy2(__file__,phase.path/'closeout.py')
try:
    assert gate['status']=='CHECKS-COMPLETE-REVIEW-REQUIRED'
    assert audit['status']=='PREPARATION-AND-OWNERS-COMPLETE-REVIEW-REQUIRED'
    assert review['status']=='ALL-SAVED-OWNERS-AND-JOINT-WINDOWS-VERIFIED'
    source=fingerprint()
    assert source==gate['source_fingerprint']==audit['source_fingerprint']==review['source_fingerprint']
    assert all(digest(TREE/name)==sha for name,sha in gate['source_files'].items())
    assert all(digest(path)==sha for path,sha in audit['dependencies'].items())
    for name in ('q1-owners-1','q1-prepare-1'):
        assert all(digest(path)==sha for path,sha in load(ROUND/name/'STATE.json')['dependencies'].items())
    identities=prior_identities()
    assert identities==gate['prior_identities']==audit['prior_identities']
    phase.state.update(source_fingerprint=source,identities=identities,
        gate=args.gate,audit=args.audit,review=args.review,
        remaining='High-cardinality/union/recursive/aggregate allocation controls and matched ordinary prepare+cold/second/warm timing; no speed acceptance, full trace, commit, push or release.')
    phase.save()
    phase.run('diff-check',['git','-C',TREE,'diff','--check'])
    phase.run('format',['cargo','fmt','--manifest-path',TREE/'Cargo.toml','--all','--check'])
    assert fingerprint()==source and prior_identities()==identities
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('GATED-Q1-SOURCE-RESTORED-PRIOR-CANDIDATES-AND-EVIDENCE-VERIFIED')
