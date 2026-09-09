#!/usr/bin/env python3
"""Source-derived first-use map work from already recorded owners, not a new capture."""
import ast
import json
import re
import shutil
from diagnostics import Phase, ROUND, digest, load
from q1_experiment import TREE, fingerprint, prior_identities

phase = Phase('q1-growth-model-1')
shutil.copy2(__file__, phase.path/'model.py')
families = {'computed', 'union', 'groups', 'pairs', 'union_groups', 'dnf_groups'}
pattern = re.compile(r'^MAP (\S+) role=(\S+) rows=(\d+) arity=(\d+) owners=(\[.*?\]) bytes=(\d+)$', re.M)

def records(path):
    result = {}
    for label, role, n, width, raw, size in pattern.findall(path.read_text()):
        case_window, owner = label.split('/', 1)
        family, bound, window = case_window.split(':')
        if family not in families or window not in {'prepare', 'cold'}:
            continue
        owners = ast.literal_eval(raw)
        cap = owners[2][0]  # Vec<()> capacity is usize::MAX; its length is the slot count.
        arity, n, size = int(width), int(n), int(size)
        assert owners[0][0] == (cap+7 if cap else 0)
        assert owners[1][0] == cap*arity and owners[3][0] == cap
        assert owners[4][0] == n and n*3 <= cap
        assert size == sum(c*w for _, c, w in owners)
        result[family+':'+bound, owner, role, window] = dict(
            slots=cap, rows=n, arity=arity, value_bytes=owners[2][2], owners=owners)
    return result

def model(initial, actual):
    cap, n = initial['slots'], actual['rows']
    arity, value = actual['arity'], actual['value_bytes']
    def backing(c):
        return (c+7) + c*arity*8 + c*value + c if c else 0
    steps = []
    requested = backing(cap)
    requests = (2+int(arity > 0)+int(value > 0)) if cap else 0
    while n*3 > cap:
        following = max(8, cap*2)
        steps.append(dict(old_slots=cap, new_slots=following, rehashed_entries=cap//3,
                          rehashed_key_words=(cap//3)*arity,
                          copied_value_bytes=(cap//3)*value, requested_bytes=backing(following)))
        requested += backing(following)
        requests += 2+int(arity > 0)+int(value > 0)
        cap = following
    assert cap == actual['slots'], (initial, actual, cap)
    return dict(steps=steps, table_requests=requests, table_requested_bytes=requested,
                rehashed_entries=sum(s['rehashed_entries'] for s in steps),
                rehashed_key_words=sum(s['rehashed_key_words'] for s in steps),
                copied_value_bytes=sum(s['copied_value_bytes'] for s in steps))

try:
    source = fingerprint()
    assert source == load(ROUND/'q1-gates-2/STATE.json')['source_fingerprint']
    identities = prior_identities()
    paths = {v:ROUND/f'q1-controls-{v}-2/controls.log' for v in ['baseline', 'candidate']}
    observed = {v:records(p) for v, p in paths.items()}
    assert observed['baseline'].keys() == observed['candidate'].keys()
    models = {}
    cases = {}
    for family_bound, owner, role, window in observed['baseline']:
        if window != 'cold':
            continue
        key = family_bound+'/'+owner+'/'+role
        pair = {}
        for variant in ['baseline', 'candidate']:
            initial = observed[variant][family_bound, owner, role, 'prepare']
            actual = observed[variant][family_bound, owner, role, 'cold']
            assert initial['rows'] == 0
            pair[variant] = model(initial, actual)
        fields = ['table_requests', 'table_requested_bytes', 'rehashed_entries',
                  'rehashed_key_words', 'copied_value_bytes']
        delta = {f:pair['candidate'][f]-pair['baseline'][f] for f in fields}
        pair['delta'] = delta
        models[key] = pair
        total = cases.setdefault(family_bound, {f:0 for f in fields})
        for f in fields:
            total[f] += delta[f]
        if family_bound in ['union:100000', 'computed:100000', 'dnf_groups:500', 'groups:8192']:
            print(key, 'baseline', {k:v for k,v in pair['baseline'].items() if k!='steps'},
                  'candidate', {k:v for k,v in pair['candidate'].items() if k!='steps'}, 'delta', delta)
    allocation = load(ROUND/'q1-controls-review-1/STATE.json')
    for case, data in cases.items():
        measured = allocation['summary'][case]['combined_delta']
        data['measured_all_requested_bytes_delta'] = measured[2]
        data['requested_delta_outside_modeled_tables'] = measured[2]-data['table_requested_bytes']
        data['measured_all_requests_delta'] = measured[0]
        data['request_delta_outside_modeled_tables'] = measured[0]-data['table_requests']
    phase.state.update(source_fingerprint=source, prior_identities=identities,
        source_files={str(TREE/p):digest(TREE/p) for p in [
            'crates/bumbledb/src/exec/wordmap.rs', 'crates/bumbledb/src/exec/wordmap/grow.rs',
            'crates/bumbledb/src/exec/wordmap/entry.rs', 'crates/bumbledb/src/exec/wordmap/new.rs']},
        input_hashes={str(p):digest(p) for p in paths.values()}, maps=models, cases=cases,
        assumptions='Single monotonic first-use fill for these nonrecursive control maps; load denominator 3, doubling from minimum8, successful inserts, no mid-fill clear/release/spill. Cardinalities and initial/final geometry are observed; intermediate work is source-derived, not a measured event trace. No prediction of wall time/cache misses/RSS.',
        scope='Backing arrays only: excludes dense-list growth and other owners/transient preparation allocations. Residual against complete measured allocation windows is retained, not silently attributed to table growth.')
    assert fingerprint() == source and prior_identities() == identities
    phase.save()
except BaseException:
    phase.finish('INCOMPLETE'); raise
else:
    phase.finish('SAVED-OWNER-GROWTH-MODEL-COMPLETE-ASSUMPTIONS-EXPLICIT')
