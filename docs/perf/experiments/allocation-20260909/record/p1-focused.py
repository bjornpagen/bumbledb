#!/usr/bin/env python3
"""Bounded ABBA ordinary controls, using the already verified frozen binaries."""
import argparse
import shutil
from diagnostics import Phase, REPO, ROUND, digest, load

FAMILIES = ['point', 'range', 'stats', 'triangle', 'disp_probe']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('attempt', type=int)
    args = parser.parse_args()
    phase = Phase(f'p1-focused-{args.attempt}')
    shutil.copy2(__file__, phase.path / 'focused.py')
    original = load(ROUND / 'p1-evaluation-2/STATE.json')
    resume = load(ROUND / 'p1-evaluation-2-resume-1/STATE.json')
    assert resume['status'] == 'MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
    kernels = REPO / 'crates/bumbledb/src/exec/run/probe_pass.rs'
    kernel_hash = digest(kernels)
    phase.state.update(
        diagnostic_only=False,
        hypothesis='Investigate undisplaced root-probe signal and replace flagged cheap controls.',
        protocol='Ordinary ABBA, 32 samples per family, read batch 1; no profiler or new trace.',
        source_kernel_sha256=kernel_hash,
        limitations='Descriptive shared-host controls, no hard P-core affinity or Pi qualification.',
    )
    phase.save()
    try:
        for label in ['A0', 'B0', 'B1', 'A1']:
            baseline = label.startswith('A')
            binary = (ROUND / 'release/bumbledb-bench' if baseline
                      else ROUND / 'p1-evaluation-2/candidate-bumbledb-bench')
            data = (ROUND / 'p1-evaluation-2/data' if baseline
                    else ROUND / 'p1-evaluation-2-resume-1/candidate-data')
            assert digest(binary) == original['baseline_sha256' if baseline else 'candidate_sha256']
            assert digest(kernels) == kernel_hash
            out = phase.path / label
            report_path = out / 'report.json'
            phase.run(label, [binary, 'bench', '--samples', '32', '--read-batch', '1',
                              '--families', ','.join(FAMILIES), '--dir', data, '--out', out],
                      report_path)
            report = load(report_path)
            assert report['config'] == dict(scale='S', seed=1, samples=32, store='durable')
            assert [r['name'] for r in report['reads']] == FAMILIES
            assert not report['verify_stamp'].startswith('UNVERIFIED')
            assert report['corpus_digest'] == load(ROUND / 'full/reads/report.json')['corpus_digest']
            for row in report['reads']:
                assert row['batch'] == 1
                print(label, row['name'], row['ours'], row.get('ghz_ours'), flush=True)
        assert digest(kernels) == kernel_hash
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')


if __name__ == '__main__':
    main()
