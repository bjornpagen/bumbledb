"""Read-only reachable-root diagnostics. Never use these processes for timings."""
from pathlib import Path
import argparse
import hashlib
import json
import os
from run import LAB, RESULTS, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--output', required=True)
ap.add_argument('--candidate', nargs='+', choices=['prefix64','prefix512','retraction64','retraction512'], default=['prefix512'])
ap.add_argument('--normalize', nargs='+', choices=['staged','equal','ite'], default=['ite'])
ap.add_argument('--family', nargs='+', choices=['suffix-1','suffix-half','non-suffix'], default=['non-suffix'])
ap.add_argument('--layout', nargs='+', choices=['bit-major','face-major'], default=['bit-major'])
ap.add_argument('--gates',choices=['joint','active','witness'],default='witness')
args = ap.parse_args()
check_engine_sources()
build = json.loads((RESULTS/'build.json').read_text())
assert hashlib.sha256(Path(build['binary']).read_bytes()).hexdigest() == build['binary_sha256']
for name, digest in build['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
os.environ['RUST_BACKTRACE'] = '1'
common = dict(EVENT_LAB_LANE='readouts', EVENT_LAB_ESSENTIAL_KERNEL='derived',
              EVENT_LAB_ESSENTIAL_LAYOUT='slab', EVENT_LAB_OCCUPANCY_KERNEL='words',
              EVENT_LAB_PRODUCT='materialized', EVENT_LAB_LEGAL_DOMAIN='fibred',
              EVENT_LAB_READOUT_FACES='x', EVENT_LAB_RETRACTION_COUNT='words',
              EVENT_LAB_RETRACTION_FACTOR='faces', EVENT_LAB_REACHABILITY='1',
              EVENT_LAB_RETRACTION_PROJECT=args.gates)
runs = []
for candidate in args.candidate:
    for family in args.family:
        for layout in args.layout:
            for mode in args.normalize:
                job = dict(common, EVENT_LAB_CANDIDATE=candidate, EVENT_LAB_LAYOUT=layout,
                           EVENT_LAB_READOUT_FAMILY=family, EVENT_LAB_RETRACTION_NORMALIZE=mode)
                result = execute(build['binary'], job, len(runs), 1, 240, folder)
                runs.append(result)
                for row in result['rows']:
                    if row.get('kind') != 'readout_reachability': continue
                    g = row['graphs']; width = row['width']
                    k = 9 if candidate.endswith('512') else 6
                    assert len(g['output_masks']) == len(g['output_records']) == 64
                    legal_axes = ((1 << (2*width))-1) | (1 << (3*width))
                    for mask, size in zip(g['output_masks'], g['output_records']):
                        assert mask & ~legal_axes == 0, 'Unexpected scratch-coordinate dependence'
                        dimension = bin(mask).count('1')
                        bound = 1 if dimension <= k else (1 << (dimension-k+1))-1
                        assert size <= bound, 'Reduced binary/table graph bound'
                    assert g['live_union']['records'] <= g['arena_records']
                    assert g['outputs']['records'] <= g['live_union']['records']
                record = dict(build_metadata=build, runs=runs,
                    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    boundary='Graph censuses precede canonical reimport; timings excluded. A capped process has no complete semantic acceptance. Live union excludes cache entries but includes decoder/count infrastructure, source inputs and output roots.')
                (RESULTS/args.output).write_text(json.dumps(record, indent=2)+'\n')
                if result['status'] == 'failed': raise SystemExit('Diagnostic assertion failed')
