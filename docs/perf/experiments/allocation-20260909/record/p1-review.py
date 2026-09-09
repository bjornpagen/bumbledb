#!/usr/bin/env python3
"""Review only completed P1 timing receipts. Never accepts a candidate."""

import argparse
import math

from diagnostics import ROUND, digest, load

ORIGINAL = ROUND / 'p1-evaluation-2'
RESUME = ROUND / 'p1-evaluation-2-resume-1'
LABELS = ['A0', 'A1', 'B0', 'A2', 'B1', 'A3']


def receipts():
    records = {}
    original = load(ORIGINAL / 'STATE.json')
    for step in original['steps']:
        if step['status'] == 'PASS' and 'artifact' in step:
            assert digest(step['artifact']) == step['sha256']
            records[step['name']] = step
    if (RESUME / 'STATE.json').exists():
        resumed = load(RESUME / 'STATE.json')
        for field in ['source_fingerprint', 'baseline_sha256', 'candidate_sha256']:
            assert resumed[field] == original[field]
        for step in resumed['steps']:
            if step['status'] == 'PASS' and 'artifact' in step:
                assert step['name'] not in records, 'successful measurement was repeated'
                assert digest(step['artifact']) == step['sha256']
                records[step['name']] = step
    return records


def panel(records, kind, metrics, families):
    reports = {}
    roster = None
    for label in LABELS:
        if label + '-' + kind not in records:
            continue
        report = load(records[label + '-' + kind]['artifact'])
        rows = report['queries' if kind == 'scenarios' else 'reads']
        if kind == 'scenarios':
            assert report['samples'] == 24 and report['warmups'] == 8
        else:
            assert report['config']['samples'] == 96
            assert all(row['batch'] == 1 for row in rows)
            assert not report['verify_stamp'].startswith('UNVERIFIED')
        names = [row['name'] for row in rows]
        if roster is None:
            roster = names
        assert names == roster
        reports[label] = {row['name']: row for row in rows}
    if not reports:
        return
    print('\n' + kind + ': completed ' + ', '.join(reports))
    print('family / statistic | A0 us | A1/A0 | B0/A1 | B0/sqrt(A1*A2) | '
          'B1/A2 | B1/sqrt(A2*A3) | A max/min')
    for name in roster:
        if kind == 'scenarios':
            assert len({rows[name]['answers'] for rows in reports.values()}) == 1, name
        if families and name not in families:
            continue
        clock_flagged = {label for label, rows in reports.items()
                         if (rows[name].get('ghz_ours') or {}).get('contaminated')}
        for metric in metrics:
            values = {label: rows[name]['ours'][metric] for label, rows in reports.items()}
            controls = [value for label, value in values.items() if label.startswith('A')]

            def relative(candidate, left, right=None):
                labels = [candidate, left] + ([right] if right else [])
                if not all(label in values for label in labels):
                    return 'pending'
                denominator = (math.sqrt(values[left] * values[right])
                               if right else values[left])
                if not denominator:
                    return 'undefined'
                flag = '!' if clock_flagged.intersection(labels) else ''
                return f'{values[candidate] / denominator:.4f}{flag}'

            spread = (f'{max(controls) / min(controls):.4f}'
                      if len(controls) > 1 and min(controls) else 'undefined')
            print(f'{name} / {metric} | {values["A0"] / 1000:.3f} | '
                  f'{relative("A1", "A0")} | {relative("B0", "A1")} | '
                  f'{relative("B0", "A1", "A2")} | {relative("B1", "A2")} | '
                  f'{relative("B1", "A2", "A3")} | {spread}')
    if kind == 'reads':
        for label, rows in reports.items():
            for name, row in rows.items():
                clock = row.get('ghz_ours')
                if clock and (clock.get('retried') or clock.get('contaminated')):
                    print(f'CLOCK FLAG {label}/{name}: {clock}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--metrics', nargs='+', default=['p50'],
                        choices=['min', 'p50', 'mean_ns', 'p90', 'p95', 'p99', 'max'])
    parser.add_argument('--families', nargs='+')
    args = parser.parse_args()
    completed = receipts()
    print('P1 descriptive control review; ratios are not an acceptance test.')
    print('Shared-host effects and rotating parameter distributions require full report review.')
    print('One-sided ratios are preliminary; p99 is the maximum at 24/96 samples.')
    print('! = includes a contaminated engine clock window; not usable for acceptance.')
    panel(completed, 'scenarios', args.metrics, args.families)
    panel(completed, 'reads', args.metrics, args.families)
