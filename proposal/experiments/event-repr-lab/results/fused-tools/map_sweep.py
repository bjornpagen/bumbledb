"""Same-binary, shuffled serial comparison of packed permutation algorithms."""
from pathlib import Path
import argparse
import hashlib
import json
import random

from run import CANDIDATES, LAB, RESULTS, check_engine_sources, execute


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--output', required=True)
    ap.add_argument('--trials', type=int, default=11)
    ap.add_argument('--seed', type=int, default=20260919)
    ap.add_argument('--timeout', type=int, default=60)
    ap.add_argument('--candidate', nargs='+', choices=CANDIDATES, default=CANDIDATES)
    ap.add_argument('--layout', nargs='+', choices=['face-major', 'bit-major', 'pair-major'], default=['face-major', 'bit-major', 'pair-major'])
    args = ap.parse_args()
    check_engine_sources()
    metadata = json.loads((RESULTS / 'build.json').read_text())
    binary = Path(metadata['binary'])
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == metadata['binary_sha256']
    for name, digest in metadata['lab_sources'].items():
        assert hashlib.sha256((LAB / name).read_bytes()).hexdigest() == digest, name
    log_dir = RESULTS / (Path(args.output).stem + '-logs')
    log_dir.mkdir(exist_ok=False)
    (log_dir / 'build.json').write_text(json.dumps(metadata, indent=2) + '\n')
    records = []

    def save():
        (RESULTS / args.output).write_text(json.dumps({
            'experiment': 'Same-binary packed permutation control; finite and diagram controls included',
            'seed': args.seed, 'trials': args.trials, 'timeout_s': args.timeout,
            'build_metadata': metadata, 'runs': records,
        }, indent=2) + '\n')

    for policy in ['local', 'recursive']:
        record = execute(str(binary), {'verify': True, 'EVENT_LAB_PACKED_MAP': policy},
                         len(records), args.trials, args.timeout, log_dir)
        records.append(record)
        save()
        if record['status'] != 'passed':
            raise SystemExit('Semantic checks failed: ' + record['log'])
    jobs = []
    for candidate in args.candidate:
        for layout in args.layout:
            for policy in ['local', 'recursive'] if candidate.startswith('packed') else ['local']:
                jobs.append({'EVENT_LAB_LANE': 'relations', 'EVENT_LAB_CANDIDATE': candidate,
                             'EVENT_LAB_LAYOUT': layout, 'EVENT_LAB_PACKED_MAP': policy})
    random.Random(args.seed).shuffle(jobs)
    for job in jobs:
        record = execute(str(binary), job, len(records), args.trials, args.timeout, log_dir)
        records.append(record)
        save()
        if record['status'] == 'failed':
            raise SystemExit('Candidate failed: ' + record['log'])
    print('Saved ' + str(RESULTS / args.output), flush=True)


if __name__ == '__main__':
    main()
