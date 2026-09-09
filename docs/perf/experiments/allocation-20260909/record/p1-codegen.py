#!/usr/bin/env python3
"""Inspect frozen P1 symbols serially, retaining raw and normalized assembly."""
import argparse
import difflib
import json
import re
import shutil

from diagnostics import Phase, ROUND, digest, load


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--iterator', action='store_true')
    args = parser.parse_args()
    phase = Phase('p1-codegen-iterator-1' if args.iterator else 'p1-codegen-1')
    shutil.copy2(__file__, phase.path / 'inspect.py')
    binaries = {
        'baseline': ROUND / 'release/bumbledb-bench',
        'candidate': ROUND / 'p1-evaluation-2/candidate-bumbledb-bench',
    }
    expected = {
        'baseline': '84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74',
        'candidate': 'ac4f45aba133369710b2d5a1ac458503fe1d60c7ba5e3f1f12236fff609fa622',
    }
    if args.iterator:
        state = load(ROUND / 'p1-iterator-1/STATE.json')
        assert state['status'] == 'GATES-COMPLETE-CODEGEN-AND-PERFORMANCE-REVIEW-REQUIRED'
        binaries['candidate'] = ROUND / 'p1-iterator-1/bumbledb-bench'
        expected['candidate'] = state['binary_sha256']
    result = {}
    try:
        for label, binary in binaries.items():
            assert digest(binary) == expected[label]
            phase.run(label + '-symbols', ['xcrun', 'nm', '-n', binary])
            names = re.findall(r'^\S+ t (\S*probe_sibling_keysKj1_\S*)$',
                               (phase.path / (label + '-symbols.log')).read_text(), re.M)
            assert len(names) == 4, names
            result[label] = {}
            for name in names:
                kind = 'callback' if name.startswith('__RNC') else 'function'
                key = kind + ('-children' if 'Kb1_' in name else '-presence')
                phase.run(label + '-' + key,
                          ['xcrun', 'llvm-objdump', '-d', '--no-show-raw-insn',
                           '--disassemble-symbols=' + name, binary])
                raw = (phase.path / (label + '-' + key + '.log')).read_text()
                base = int(re.search(r'^([0-9a-f]+) <', raw, re.M)[1], 16)
                instructions = []
                for line in raw.splitlines():
                    match = re.match(r'([0-9a-f]+):\s+(.*)', line)
                    if not match:
                        continue
                    insn = match[2]
                    # Keep local branch offsets, symbolic external calls and all
                    # register/immediate choices. Do not hide data addressing.
                    insn = re.sub(r'0x[0-9a-f]+ <([^>]+)>',
                                  lambda m: '<' + m[1].replace(name, 'self') + '>', insn)
                    instructions.append(f'{int(match[1], 16) - base:04x}: {insn}')
                result[label][key] = instructions
                (phase.path / (label + '-' + key + '.asm')).write_text(
                    '\n'.join(instructions) + '\n')
        summary = {}
        for key in result['baseline']:
            a, b = result['baseline'][key], result['candidate'][key]
            delta = ''.join(difflib.unified_diff(
                [s + '\n' for s in a], [s + '\n' for s in b],
                fromfile='baseline', tofile='candidate'))
            (phase.path / (key + '.diff')).write_text(delta)
            summary[key] = dict(baseline_instructions=len(a),
                                candidate_instructions=len(b), normalized_equal=a == b)
        (phase.path / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
        print(json.dumps(summary, indent=2))
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('COMPLETE')


if __name__ == '__main__':
    main()
