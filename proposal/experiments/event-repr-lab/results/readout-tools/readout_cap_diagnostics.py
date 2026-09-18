"""Repeat capped readout jobs serially with backtraces; do not rank these runs."""
from pathlib import Path
import argparse
import hashlib
import json
import os
from run import LAB, RESULTS, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('input')
ap.add_argument('--output', required=True)
args = ap.parse_args()
source = json.loads((RESULTS/args.input).read_text())
build = json.loads((RESULTS/'build.json').read_text())
assert source['build_metadata'] == build
check_engine_sources()
binary = Path(build['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == build['binary_sha256']
for name, digest in build['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
os.environ['RUST_BACKTRACE'] = 'full'
runs = []
for previous in source['runs']:
    if previous['status'] != 'resource_cap':
        continue
    result = execute(str(binary), previous['job'], len(runs), 1, 180, folder)
    log = (RESULTS/result['log']).read_text()
    result['cap_in_canonical_reimport'] = (
        result['status'] == 'resource_cap' and 'readout::verify_batch' in log
        and 'RegionOps>::import' in log)
    runs.append(result)
    record = dict(input=args.input, build_metadata=build, runs=runs,
                  checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  environment={'RUST_BACKTRACE':'full'},
                  boundary='Untimed-failure diagnosis. These processes are excluded from comparative timings.')
    (RESULTS/args.output).write_text(json.dumps(record, indent=2)+'\n')
assert runs, 'Input has no capped jobs'
print(json.dumps([dict(candidate=r['job']['EVENT_LAB_CANDIDATE'],
                       layout=r['job']['EVENT_LAB_LAYOUT'],
                       family=r['job']['EVENT_LAB_READOUT_FAMILY'],
                       status=r['status'],
                       cap_in_canonical_reimport=r['cap_in_canonical_reimport']) for r in runs], indent=2))
