#!/usr/bin/env python3
import argparse
import math
import re
from diagnostics import ROUND, digest, load

parser = argparse.ArgumentParser()
parser.add_argument('attempt', type=int)
args = parser.parse_args()
path = ROUND/f'p3-filter-mechanism-{args.attempt}'
state = load(path/'STATE.json')
assert state['status'] == 'MECHANISM-COMPLETE-REVIEW-REQUIRED'
assert digest(path/'flat-mechanism') == state['binary_sha256']
rows = {}
flags = []
for line in (path/'mechanism.log').read_text().splitlines():
    match = re.fullmatch(r'MECH (\S+) (\S+) ns=(\d+) draws=(\d+) calls=(\d+) pre=([\d.]+) post=([\d.]+) flagged=(true|false) checksum=(\d+)', line)
    if match:
        name, label, ns, draws, calls, pre, post, flag, checksum = match.groups()
        rows.setdefault(name, {})[label] = int(ns)/int(calls)
        if flag == 'true': flags.append((name, label, pre, post))
assert len(rows) == 10 and all(len(row) == 6 for row in rows.values())
print('| Case | B/A | C/A | A0/A1 ns/call | B0/B1 ns/call | C0/C1 ns/call |')
print('|---|---:|---:|---:|---:|---:|')
for name, row in rows.items():
    centers = {key: math.sqrt(row[key+'0']*row[key+'1']) for key in 'ABC'}
    print(f'| {name} | {centers["B"]/centers["A"]:.4f} | {centers["C"]/centers["A"]:.4f} | '+
          ' | '.join(f'{row[key+"0"]:.4f}/{row[key+"1"]:.4f}' for key in 'ABC')+' |')
print('FLAGGED', len(flags), flags)
print('No normalization, omitted windows or significance claim. Micro mechanism only.')
