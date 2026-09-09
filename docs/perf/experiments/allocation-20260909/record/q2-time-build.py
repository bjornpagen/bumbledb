#!/usr/bin/env python3
"""Fresh same-source-path ordinary builds; explicitly separate both Cargo caches."""
import argparse
import json
import shutil
from diagnostics import ENV,Phase,ROUND,digest
from q2_time_experiment import TREE,GATES,git,source,dependencies
from q2_experiment import prior_identities
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant',choices=['q1','q2'])
parser.add_argument('attempt',type=int)
args=parser.parse_args()
files=source(args.variant);deps=dependencies()
phase=Phase(f'q2-time-build-{args.variant}-{args.attempt}')
phase.state.update(variant=args.variant,source_tree=str(TREE),source_files=files,
    candidate_identity=GATES['q2']['source_fingerprint'],ordinary_source=GATES[args.variant]['source_fingerprint'],dependencies=deps,prior_identities=prior_identities(),
    protocol='Same checkout path, same external driver, workspace ordinary fat-LTO release profile; no allocator/CPU tracing; fresh phase-specific target AND build directories.')
phase.save();shutil.copy2(__file__,phase.path/'build.py')
shutil.copy2(ROUND/'q2_time_experiment.py',phase.path/'q2_time_experiment.py')
ENV.update(CARGO_BUILD_JOBS='1',CARGO_TARGET_DIR=str(phase.path/'cargo-target'),CARGO_BUILD_BUILD_DIR=str(phase.path/'cargo-build'))
def cargo(action,*extra):return ['cargo',action,'--manifest-path',TREE/'Cargo.toml',*extra]
try:
    for path in deps:shutil.copy2(path,phase.path/('dependency-'+str(len(list(phase.path.glob('dependency-*'))))+'-'+path.rsplit('/',1)[-1]))
    for name in files:
        dest=phase.path/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(TREE/name,dest)
    phase.run('source-patch',['git','-C',TREE,'diff','--binary','HEAD'])
    phase.run('format',cargo('fmt','--all','--check'))
    phase.run('clippy',cargo('clippy','--locked','-p','bumbledb-bench','--bin','q1-timing','--','-D','warnings'))
    phase.run('build',cargo('build','--locked','--release','-p','bumbledb-bench','--bin','q1-timing','--message-format=json'))
    artifacts=[json.loads(line) for line in (phase.path/'build.log').read_text().splitlines() if line.startswith('{') and json.loads(line).get('reason')=='compiler-artifact']
    engine=[a for a in artifacts if a['target']['name']=='bumbledb'];binary=[a for a in artifacts if a['target']['name']=='q1-timing' and a['executable']]
    assert len(engine)==len(binary)==1
    assert engine[0]['features']==['collision-probe'] and not engine[0]['fresh'] and not binary[0]['fresh']
    assert engine[0]['profile']['opt_level']=='3' and not engine[0]['profile']['debug_assertions'] and not engine[0]['profile']['test']
    assert engine[0]['package_id'].startswith('path+file://'+str(TREE)+'/crates/bumbledb#')
    assert binary[0]['executable'].startswith(str(phase.path)+'/')
    frozen=phase.path/'q1-timing';shutil.copy2(binary[0]['executable'],frozen)
    phase.state.update(engine_artifact=engine[0],binary_artifact=binary[0],binary_sha256=digest(frozen))
    phase.save()
    assert source(args.variant)==files and dependencies()==deps and prior_identities()==phase.state['prior_identities']
except BaseException:
    phase.finish('INCOMPLETE');raise
else:phase.finish('ORDINARY-TIMING-BINARY-FROZEN')
