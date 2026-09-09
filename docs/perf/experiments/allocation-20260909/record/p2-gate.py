#!/usr/bin/env python3
"""Gate and freeze standalone P2; no timing or profiler is launched."""
import argparse
import hashlib
import shutil
import subprocess

from diagnostics import ENV, Phase, REPO, ROUND, SOURCE, digest, load


def git(*args):
    return subprocess.check_output(['git', *args], cwd=REPO)


def fingerprint():
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert not git('diff', 'HEAD', '--', 'crates/bumbledb/src/exec/run/probe_pass.rs'), \
        'Do not bundle unaccepted P1 with P2'
    patch = git('diff', '--binary', 'HEAD')
    untracked = git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')
    result = hashlib.sha256(patch)
    for name in sorted(filter(None, untracked)):
        assert name.startswith(b'crates/bumbledb/'), name
        result.update(name + b'\0')
        result.update(bytes.fromhex(digest(REPO / name.decode())))
    return result.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('attempt', type=int)
    args = parser.parse_args()
    assert args.attempt > 0
    assert load(ROUND / 'p1-variants-1/STATE.json')['status'] == \
        'MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
    ENV['CARGO_BUILD_JOBS'] = '1'
    source_hash = fingerprint()
    phase = Phase(f'p2-gates-{args.attempt}')
    shutil.copy2(__file__, phase.path / 'gate.py')
    phase.state.update(
        source_fingerprint=source_hash,
        source_note='Published baseline plus isolated P2 and retained test-only repairs.',
        hypothesis='Read witnessed aggregate inputs directly without full-binding staging; '
                   'preserve existing dedup ownership, amortized outer reads, and bulk reductions.',
        acceptance='Correctness gates only. No speedup, RSS, or Pi qualification implied.',
    )
    phase.save()
    try:
        phase.run('source-patch', ['git', 'diff', '--binary', 'HEAD'])
        for name in filter(None, git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')):
            dest = phase.path / 'untracked' / name.decode()
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPO / name.decode(), dest)
        commands = [
            ('format', ['cargo', 'fmt', '--all', '--check']),
            ('clippy', ['cargo', 'clippy', '--locked', '-p', 'bumbledb', '--all-targets',
                        '--', '-D', 'warnings']),
            ('library-tests', ['cargo', 'test', '--locked', '-p', 'bumbledb', '--lib',
                               '--', '--quiet', '--test-threads=1']),
            ('allocation-tests', ['cargo', 'test', '--locked', '-p', 'bumbledb',
                                  '--features', 'alloc-counter', '--lib', 'exec::',
                                  '--', '--quiet', '--test-threads=1']),
            ('release-build', ['cargo', 'build', '--locked', '--release', '-p', 'bumbledb-bench']),
        ]
        for name, command in commands:
            assert fingerprint() == source_hash
            phase.run(name, command)
        binary = phase.path / 'bumbledb-bench'
        shutil.copy2(REPO / 'target/release/bumbledb-bench', binary)
        phase.state['binary_sha256'] = digest(binary)
        phase.save()
        phase.run('verify-candidate', [binary, 'verify', '--dir', phase.path / 'data'])
        assert fingerprint() == source_hash
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('GATES-COMPLETE-PERFORMANCE-REVIEW-REQUIRED')


if __name__ == '__main__':
    main()
