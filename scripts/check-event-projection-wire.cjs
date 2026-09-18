#!/usr/bin/env node
// Load the just-built addon in an isolated temporary directory. This checks
// the actual N-API projection grammar; no installed package is replaced.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');

const binary = path.resolve(process.argv[2]);
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'bumbledb-event-projection-'));
try {
  const addon = path.join(temp, 'bumbledb.node');
  fs.copyFileSync(binary, addon);
  const native = require(addon);
  const full = { event: 'full' };
  const spec = {
    relations: [
      { name: 'Roster', fields: [{ name: 'group', valueType: { kind: 'u64' } }], closed: undefined },
      { name: 'Branch', fields: [
        { name: 'group', valueType: { kind: 'u64' } },
        { name: 'when', valueType: { kind: 'event' } },
      ], closed: undefined },
    ],
    statements: [
      { kind: 'fd', relation: 'Roster', projection: ['group', full] },
      { kind: 'fd', relation: 'Branch', projection: ['group', 'when'] },
      { kind: 'containment', bidirectional: true,
        source: { relation: 'Roster', projection: ['group', full], selection: [] },
        target: { relation: 'Branch', projection: ['group', 'when'], selection: [] } },
    ],
  };
  const sealed = native.descriptor(spec);
  assert.deepEqual(sealed.statements[0].projection, [0, full]);
  assert.deepEqual(sealed.statements[1].projection, [0, 1]);
  assert.deepEqual(sealed.statements[2].source.projection, [0, full]);
  assert.deepEqual(sealed.statements[2].target.projection, [0, 1]);
  assert.deepEqual(sealed.statements[3].target.projection, [0, full]);
  const singleton = { relations: [spec.relations[0]], statements: [
    { kind: 'fd', relation: 'Roster', projection: [full] },
  ] };
  assert.deepEqual(native.descriptor(singleton).statements[0].projection, [full]);
  const scalar = { relations: singleton.relations, statements: [
    { kind: 'fd', relation: 'Roster', projection: ['group'] },
  ] };
  const fullKey = { relations: singleton.relations, statements: [spec.statements[0]] };
  assert.notEqual(native.descriptor(scalar).fingerprint, native.descriptor(fullKey).fingerprint);
  let refused = 0;
  for (const projection of [
    [full, 'group'], [full, full], [{ event: 'empty' }], [{ event: 'full', field: 0 }],
    [{}], [true], [null], [0],
  ]) {
    const malformed = { ...singleton, statements: [{ ...singleton.statements[0], projection }] };
    assert.throws(() => native.descriptor(malformed));
    refused++;
  }
  // Named fields remain names: a Boolean cell cannot stand in for the constant.
  const bool = structuredClone(spec);
  bool.relations[1].fields[1].valueType = { kind: 'bool' };
  assert.throws(() => native.descriptor(bool));
  console.log(JSON.stringify({ passed: true, malformed_markers_refused: refused,
    addon_sha256: createHash('sha256').update(fs.readFileSync(binary)).digest('hex') }));
} finally {
  fs.rmSync(temp, { recursive: true, force: true });
}
