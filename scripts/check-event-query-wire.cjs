#!/usr/bin/env node
// Exercise the actual new N-API grammar in an isolated temporary store/addon.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');

const binary = path.resolve(process.argv[2]);
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'bumbledb-event-query-'));
const addon = path.join(temp, 'bumbledb.node');
fs.copyFileSync(binary, addon);
const native = require(addon);
async function operation(start, take) {
  const ready = Promise.withResolvers();
  const handle = start(ready.resolve);
  await ready.promise;
  return take(handle);
}
const close = (verb, handle) => new Promise(resolve => verb(handle, resolve));
async function main() {
  const runtime = native.runtimeOpen({ workers: 1, queueCapacity: 8, cleanupCapacity: 32,
    ownerCapacity: 32, nativeHandleCapacity: 64, cleanupTimeoutMs: 1000 });
  let directory, db, snapshot;
  try {
    const dirpath = path.join(temp, 'owner');
    fs.mkdirSync(dirpath);
    directory = await operation(cb => native.runtimeDirectoryAcquire(runtime, dirpath, cb), native.runtimeDirectoryTake);
    const spec = { relations: [{ name: 'Region', fields: [{ name: 'when', valueType: { kind: 'event' } }], closed: undefined }], statements: [] };
    const opened = await operation(cb => native.runtimeDirectoryDbOpen(directory, 'db', spec, true, cb), native.runtimeDbTake);
    assert.equal(opened.tag, 'accepted');
    db = opened.db;
    snapshot = (await operation(cb => native.runtimeDbSnapshot(db, cb), native.runtimeSnapshotTake)).snapshot;
    const a = { kind: 'var', var: 0 };
    const expressions = [a, { kind: 'empty', var: 0 }, { kind: 'full', var: 0 },
      { kind: 'not', expr: a }, { kind: 'ite', condition: a, high: a, low: a },
      { kind: 'cardinality', minimum: 2n, maximum: 2n, events: [a, a] },
      ...Array.from({ length: 16 }, (_, bits) => ({ kind: 'apply', bits, left: a, right: a }))];
    const valid = expressions.map(expr => ({ kind: 'event', expr }));
    valid.push(...['isEmpty', 'isFull'].map(kind => ({ kind: 'test', expr: { kind, expr: a } })),
      ...['subset', 'equal', 'disjoint', 'covers'].map(kind => ({ kind: 'test', expr: { kind, left: a, right: a } })));
    const query = find => ({ kind: 'cq', interiors: [], head: [{ kind: 'compute' }], rules: [{ finds: [find],
      atoms: [{ source: { kind: 'edb', relation: 0 }, bindings: [[0, { kind: 'var', var: 0 }]] }], negated: [], conditions: [] }] });
    for (const find of valid) {
      const prepared = await operation(cb => native.runtimeSnapshotPrepare(snapshot, query(find), cb), native.runtimePreparedTake);
      await close(native.runtimePreparedClose, prepared);
    }
    const malformed = [
      { kind: 'full', var: 0, ignored: true }, { kind: 'var', var: -1 },
      { kind: 'apply', bits: 16, left: a, right: a }, { kind: 'not' },
      { kind: 'ite', condition: a, high: a },
      { kind: 'cardinality', minimum: 0n, maximum: 0n, events: [] },
      { kind: 'cardinality', minimum: -1n, maximum: 1n, events: [a] },
      { kind: 'cardinality', minimum: 0n, maximum: 1n, events: Array(4096).fill(a) },
    ];
    const cycle = { kind: 'not' }; cycle.expr = cycle; malformed.push(cycle);
    for (const expr of malformed) {
      await assert.rejects(() => operation(cb => native.runtimeSnapshotPrepare(snapshot, query({ kind: 'event', expr }), cb), native.runtimePreparedTake));
    }
    console.log(JSON.stringify({ passed: true, constructors_admitted: valid.length,
      malformed_programs_refused: malformed.length,
      addon_sha256: createHash('sha256').update(fs.readFileSync(binary)).digest('hex') }));
  } finally {
    if (snapshot) await close(native.runtimeSnapshotClose, snapshot);
    if (db) await close(native.runtimeManagedDbClose, db);
    if (directory) await close((handle, callback) => native.runtimeDirectoryClose(handle, false, callback), directory);
    await close(native.runtimeClose, runtime);
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => fs.rmSync(temp, { recursive: true, force: true }));
