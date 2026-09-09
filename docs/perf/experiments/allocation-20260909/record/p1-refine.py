#!/usr/bin/env python3
"""Gate/freeze the iterator refinement. No timing, profiler or full trace."""
import argparse
import shutil

from diagnostics import Phase, REPO, ROUND, digest, load
from p1_resume_import import source_fingerprint


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('attempt', type=int)
    args = parser.parse_args()
    phase = Phase(f'p1-iterator-{args.attempt}')
    shutil.copy2(__file__, phase.path / 'refine.py')
    source_hash = source_fingerprint()
    phase.state.update(
        source_fingerprint=source_hash,
        purpose='Iterator refinement of P1, not an accepted improvement.',
        prior_candidate_sha256=load(ROUND / 'p1-evaluation-2/STATE.json')['candidate_sha256'],
        hypothesis='Remove indexed survivor-loop bookkeeping without added buffers or changed prefetch.',
    )
    phase.save()
    try:
        phase.run('source-patch', ['git', 'diff', '--binary', 'HEAD'])
        shutil.copy2(REPO / 'crates/bumbledb/src/exec/run/tests/probe_runs.rs', phase.path / 'probe_runs.rs')
        phase.run('format', ['cargo', 'fmt', '--all', '--check'])
        phase.run('library-tests', ['cargo', 'test', '--locked', '-p', 'bumbledb', '--lib',
                                    '--', '--quiet', '--test-threads=1'])
        phase.run('allocation-tests', ['cargo', 'test', '--locked', '-p', 'bumbledb',
                                      '--features', 'alloc-counter', '--lib', 'exec::',
                                      '--', '--quiet', '--test-threads=1'])
        phase.run('clippy', ['cargo', 'clippy', '--locked', '-p', 'bumbledb', '--all-targets',
                             '--', '-D', 'warnings'])
        phase.run('build', ['cargo', 'build', '--locked', '--release', '-p', 'bumbledb-bench'])
        binary = phase.path / 'bumbledb-bench'
        shutil.copy2(REPO / 'target/release/bumbledb-bench', binary)
        phase.state['binary_sha256'] = digest(binary)
        phase.save()
        assert source_fingerprint() == source_hash
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('GATES-COMPLETE-CODEGEN-AND-PERFORMANCE-REVIEW-REQUIRED')


if __name__ == '__main__':
    main()
