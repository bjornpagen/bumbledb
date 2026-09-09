#!/usr/bin/env python3
"""Reconcile every saved Q1 owner and separate/joint allocation window."""
import argparse
import ast
import re
import shutil
from diagnostics import Phase, ROUND, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('audit', type=int)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
audit_dir = ROUND/f'q1-audit-{args.audit}'
audit = load(audit_dir/'STATE.json')
assert audit['status'] == 'PREPARATION-AND-OWNERS-COMPLETE-REVIEW-REQUIRED'
phase = Phase(f'q1-audit-review-{args.attempt}')
shutil.copy2(__file__, phase.path/'review.py')

def counters(raw, prefix):
    pairs = re.findall(prefix+r' (\S+) alloc=AllocWindow \{ ([^}]+) \}', raw)
    result = {label: tuple(map(int, re.findall(r': (\d+)', fields))) for label, fields in pairs}
    assert len(result) == len(pairs) and all(len(value) == 4 for value in result.values())
    return result

def owner_lines(raw, prefix):
    return re.findall(r'^'+prefix+r' .*$', raw, re.M)

def sinks(raw):
    pairs = re.findall(
        r'^SINK (\S+) dense=(.*?) hash_rows=(\d+) hash_owners=(\[.*?\]) hash_capacity_bytes=(\d+) scratch=(\(.*?\)) scan_rows=\(0, 0\) key_bytes=\(0, 0\) spilled=false$', raw, re.M)
    result = {}
    for label, dense, rows, owners, size, scratch in pairs:
        owners = ast.literal_eval(owners)
        assert len(owners) == 5
        assert all(length <= capacity for length, capacity, _ in owners)
        assert sum(capacity*size for _, capacity, size in owners) == int(size)
        assert owners[2][2] == 0  # Zero-sized values, not an exabyte allocation.
        result[label] = dict(dense=dense, rows=int(rows), owners=owners,
                             bytes=int(size), scratch=ast.literal_eval(scratch))
    assert len(result) == len(pairs) == len(owner_lines(raw, 'SINK'))
    return result

def add(a, b):
    return tuple(x+y for x, y in zip(a, b))

def delta(a, b):
    return tuple(y-x for x, y in zip(a, b))

try:
    assert load(ROUND/'q1-owners-review-1/STATE.json')['status'] == 'ALL-OWNERS-AND-PRIOR-TRIANGLE-COUNTS-VERIFIED'
    files = {
        'baseline_execution': ROUND/'q1-owners-1/ownership.log',
        'baseline_preparation': ROUND/'q1-prepare-1/ownership.log',
        'candidate_execution': audit_dir/'ownership.log',
        'candidate_preparation': audit_dir/'preparation.log',
    }
    raw = {name: path.read_text() for name, path in files.items()}
    windows = {name: counters(text, 'EXEC' if name.endswith('execution') else 'PREPARE')
               for name, text in raw.items()}
    table = {name: sinks(text) for name, text in raw.items()}
    labels = re.findall(r'^PASS ((?:cold|warm|rotation0|rotation1)-(?:range|triangle)-[0-3])$', raw['candidate_execution'], re.M)
    assert len(labels) == len(set(labels)) == 32
    assert set(labels) == windows['baseline_execution'].keys() == windows['candidate_execution'].keys()
    assert set(labels) == table['baseline_execution'].keys() == table['candidate_execution'].keys()
    assert windows['baseline_preparation'].keys() == windows['candidate_preparation'].keys() == {'range','triangle'}
    assert table['baseline_preparation'].keys() == table['candidate_preparation'].keys() == {'prepare-range','prepare-triangle'}

    # Compare entire rows, not selected totals, against the previously validated
    # baseline. This covers all answer owners, 100 views and 64 spare buffers.
    unchanged = {}
    for window in ('execution','preparation'):
        for prefix in ('ANSWER','VIEW','SPARE','SURVIVORS'):
            a = owner_lines(raw['baseline_'+window], prefix)
            b = owner_lines(raw['candidate_'+window], prefix)
            assert a == b, (window, prefix)
            unchanged[window+'_'+prefix] = len(a)
    assert unchanged['execution_VIEW'] == 100 and unchanged['execution_SPARE'] == 64
    assert unchanged['execution_ANSWER'] == 32

    execution = {}
    for label in labels:
        kind, family, draw = label.split('-')
        draw = int(draw)
        a, b = table['baseline_execution'][label], table['candidate_execution'][label]
        assert (a['dense'],a['rows'],a['scratch']) == (b['dense'],b['rows'],b['scratch'])
        if family == 'range':
            assert a == b
        else:
            empty_fresh = kind in ('cold','warm') and draw == 3
            slots, dense_cap = (0,0) if empty_fresh else (16,8)
            assert b['owners'] == [
                (slots+7 if slots else 0, slots+7 if slots else 0, 1),
                (slots,slots,8), (slots,2**64-1,0), (slots,slots,1),
                (b['rows'],dense_cap,4)]
            assert b['bytes'] == (0 if empty_fresh else 199)
        a_count, b_count = windows['baseline_execution'][label], windows['candidate_execution'][label]
        change = delta(a_count,b_count)
        first_nonempty = family == 'triangle' and ((kind == 'cold' and draw != 3) or (kind == 'rotation0' and draw == 0))
        # Existing growth: three payload arrays at 8 slots, then 16 slots.
        # 87 + 167 requested, old 87 freed. Dense-list growth is unchanged.
        assert change == ((6,3,254,87) if first_nonempty else (0,0,0,0)), (label,change)
        if kind in ('warm','rotation1'):
            assert b_count == (0,0,0,0)
        execution[label] = dict(baseline=a_count,candidate=b_count,delta=change,
                               result_retained_baseline=a['bytes'],result_retained_candidate=b['bytes'])
        print(f'CHECK {label} counters_delta={change} result_bytes={a["bytes"]}->{b["bytes"]}')

    preparation = {}
    combined = {}
    for family, initial_backing, nodes in [('range',589831,1),('triangle',83886087,3)]:
        a, b = windows['baseline_preparation'][family], windows['candidate_preparation'][family]
        # Delete 3 table allocations + 1 collected estimate Vec, and 8 bytes
        # of unused execution-plan estimate per node. Range also dropped its
        # original 3 arrays immediately after proving output distinctness.
        expected = (-4,-4 if family == 'range' else -1,
                    -initial_backing-16*nodes,
                    -8*nodes-(initial_backing if family == 'range' else 0))
        assert delta(a,b) == expected, (family,delta(a,b),expected)
        sink = table['candidate_preparation']['prepare-'+family]
        assert sink['bytes'] == sink['rows'] == 0
        assert sink['owners'] == [(0,0,1),(0,0,8),(0,2**64-1,0),(0,0,1),(0,0,4)]
        preparation[family] = dict(baseline=a,candidate=b,delta=delta(a,b))
        print(f'PREPARATION {family} baseline={a} candidate={b} delta={delta(a,b)}')
        for draw in range(4):
            label = f'cold-{family}-{draw}'
            joint_a = add(a,windows['baseline_execution'][label])
            joint_b = add(b,windows['candidate_execution'][label])
            joint_delta = delta(joint_a,joint_b)
            retained_delta = (joint_b[2]-joint_b[3])-(joint_a[2]-joint_a[3])
            assert retained_delta == (-8 if family == 'range' else (-83886111 if draw == 3 else -83885944))
            combined[label] = dict(baseline=joint_a,candidate=joint_b,delta=joint_delta,
                                   net_requested_minus_freed_delta=retained_delta)
            print(f'PREPARE_PLUS_FIRST {family}-{draw} baseline={joint_a} candidate={joint_b} delta={joint_delta} net_bytes_delta={retained_delta}')

    phase.state.update(audit=str(audit_dir),source_fingerprint=audit['source_fingerprint'],
        input_hashes={name:digest(path) for name,path in files.items()},
        unchanged_owner_records=unchanged,execution=execution,preparation=preparation,combined=combined,
        limitations='Requested Rust layouts and retained capacities, not RSS/peak or speed. High-cardinality, union/recursive and aggregate allocation/timing controls still required; not accepted.')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('ALL-SAVED-OWNERS-AND-JOINT-WINDOWS-VERIFIED')
