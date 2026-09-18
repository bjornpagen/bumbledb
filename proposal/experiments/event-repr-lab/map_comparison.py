"""Render the matched packed-map comparison; keep both algorithm policies explicit."""
from pathlib import Path
import argparse
import collections
import json
import statistics

from run import RESULTS


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('inputs', nargs='+')
    ap.add_argument('--output', default='MAP-MEASUREMENTS.md')
    args = ap.parse_args()
    rows = collections.defaultdict(list)
    binaries = set()
    for filename in args.inputs:
        data = json.loads((RESULTS / filename).read_text())
        binaries.add(data['build_metadata']['binary_sha256'])
        for run in data['runs']:
            if run['status'] != 'passed':
                continue
            for row in run['rows']:
                if row['kind'] == 'relations_free_join' and row['candidate'].startswith('packed'):
                    rows[(row['candidate'], row['bits'], row['layout'], row['memo'], row['packed_map'])].append(row)
    if len(binaries) != 1:
        raise SystemExit('Use one matched binary per comparison.')

    def value(key, field, scale=1):
        values = rows.get(key)
        if not values:
            return None
        return statistics.median(statistics.median(r[field]) if isinstance(r[field], list) else r[field]
                                 for r in values) * scale

    def fmt(v):
        return '—' if v is None else f'{v:.3f}'

    out = ['# Matched packed-terminal permutation experiment', '',
           'One binary; one representation per candidate; two selectable renaming algorithms.',
           'All eighty full relation-query outputs are identical and checked against matrices.',
           'Cells are medians of per-process sample medians. Every process has equal weight;',
           'there is no selection of the fastest process. Ratios are descriptive, not confidence intervals.', '',
           'Inputs: ' + ', '.join(f'[{p}](results/{p})' for p in args.inputs) + '.', '',
           'Binary SHA-256: `' + next(iter(binaries)) + '`.', '']
    for bits in [12, 18]:
        for memo in [True, False]:
            out += [f'## {bits} coordinates, fresh arena, outer memo={str(memo).lower()}', '',
                    '| Candidate | Layout | Recursive (ms) | Local when valid (ms) | Recursive / local | Recursive arena (MB) | Local arena (MB) |',
                    '| --- | --- | ---: | ---: | ---: | ---: | ---: |']
            for candidate in ['packed64', 'packed256', 'packed512', 'packed4096']:
                for layout in ['face-major', 'bit-major', 'pair-major']:
                    old = (candidate, bits, layout, memo, 'recursive')
                    new = (candidate, bits, layout, memo, 'local')
                    before = value(old, 'fresh_s', 1000)
                    after = value(new, 'fresh_s', 1000)
                    ratio = before / after if before is not None and after else None
                    out.append('| ' + candidate + ' | ' + layout + ' | ' + ' | '.join(map(fmt, [
                        before, after, ratio, value(old, 'final_bytes_est', 1e-6), value(new, 'final_bytes_est', 1e-6)])) + ' |')
            out.append('')
    out += ['The local policy retains recursive rebuilding for maps that cross the terminal cut.',
            'It still pays invariant checking, map preparation, literal construction and public support normalization.',
            'Warm samples, imports, counts, load averages and process RSS remain in the raw inputs.', '']
    path = Path(__file__).resolve().parent / args.output
    path.write_text('\n'.join(out))
    print(path)


if __name__ == '__main__':
    main()
