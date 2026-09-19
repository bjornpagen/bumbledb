#!/usr/bin/env node
// Negative-control-sensitive boundary checks: well-shaped bytes must produce a
// registered operation before semantic refusal; shared backing refuses at copy.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');
const binary = path.resolve(process.argv[2]);
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'bumbledb-event-ingress-'));
const addon = path.join(temp, 'bumbledb.node');
fs.copyFileSync(binary, addon);
const native = require(addon);
const malformed = Buffer.from([66, 69, 86, 84, 1]);
const valid = Buffer.from(fs.readFileSync(path.join(__dirname, '../crates/bumbledb/tests/fixtures/event-v2-source.hex'), 'utf8').trim(), 'hex');
const tagged = value => ({ kind: 'event', value });
const variable = { kind: 'var', var: 0 };
const spec = { relations: [{ name: 'Region', fields: [{ name: 'value', valueType: { kind: 'event' } }], closed: undefined }],
  statements: [{ kind: 'fd', relation: 'Region', projection: ['value'] }] };
const query = (binding, finds = [{ kind: 'count' }], conditions = []) => ({ kind: 'cq', interiors: [],
  head: finds.map(f => ({ kind: f.kind === 'count' ? 'aggregate' : 'compute', ...(f.kind === 'count' ? { op: 'count' } : {}) })),
  rules: [{ finds, atoms: [{ source: { kind: 'edb', relation: 0 }, bindings: [[0, binding]] }], negated: [], conditions }] });
let deferred = 0;
async function operation(start, take, failure) {
  const ready = Promise.withResolvers();
  let handle;
  // This assertion fails on the old synchronous Event/import decoder.
  assert.doesNotThrow(() => { handle = start(ready.resolve); }, 'copy must return an operation before semantic admission');
  assert.ok(handle);
  await ready.promise;
  if (failure) {
    assert.throws(() => take(handle), failure);
    deferred++;
  } else return take(handle);
}
const eventError = { _tag: 'Engine', kind: 'event' };
const close = (verb, handle) => new Promise(resolve => verb(handle, resolve));
async function main() {
  const runtime = native.runtimeOpen({ workers: 1, queueCapacity: 16, cleanupCapacity: 32,
    ownerCapacity: 16, nativeHandleCapacity: 64, cleanupTimeoutMs: 1000 });
  let directory, db, snapshot, prepared;
  const drafts = [];
  try {
    fs.mkdirSync(path.join(temp, 'owner'));
    directory = await operation(cb => native.runtimeDirectoryAcquire(runtime, path.join(temp, 'owner'), cb), native.runtimeDirectoryTake);
    const opened = await operation(cb => native.runtimeDirectoryDbOpen(directory, 'db', spec, true, cb), native.runtimeDbTake);
    assert.equal(opened.tag, 'accepted');
    db = opened.db;
    snapshot = (await operation(cb => native.runtimeDbSnapshot(db, cb), native.runtimeSnapshotTake)).snapshot;
    const baseline = native.runtimeInspect(runtime).retained;
    await operation(cb => native.runtimeEncodeRows(runtime, spec, 0, 1n, [malformed], cb), native.runtimeBytesTake, eventError);
    await operation(cb => native.runtimeSnapshotGet(snapshot, 0, 0, [malformed], cb), native.runtimeRowTake, eventError);
    for (const verb of [native.runtimeDraftInsert, native.runtimeDraftDelete]) {
      const draft = await operation(cb => native.runtimeDraftOpen(runtime, spec, cb), native.runtimeDraftTake);
      drafts.push(draft);
      await operation(cb => native.runtimeDraftInsert(draft, 0, 1n, [valid], cb), native.runtimeReportTake);
      await operation(cb => verb(draft, 0, 1n, [malformed], cb), native.runtimeReportTake, eventError);
      await operation(cb => native.runtimeDraftFinish(draft, cb), native.runtimeChangesTake, { _tag: 'SpentHandle' });
    }
    const scalarParam = query({ kind: 'param', param: 0 });
    const setParam = query({ kind: 'paramSet', param: 0 });
    for (const [ir, params] of [[scalarParam, [tagged(malformed)]], [setParam, [{ kind: 'set', values: [tagged(valid), tagged(malformed)] }]]]) {
      await operation(cb => native.runtimeSnapshotExecute(snapshot, ir, params, cb), native.runtimeResultTake, eventError);
    }
    prepared = await operation(cb => native.runtimeSnapshotPrepare(snapshot, scalarParam, cb), native.runtimePreparedTake);
    await operation(cb => native.runtimePreparedExecute(prepared, [tagged(malformed)], cb), native.runtimeResultTake, eventError);
    const badLiteral = query({ kind: 'literal', value: tagged(malformed) });
    await operation(cb => native.runtimeSnapshotPrepare(snapshot, badLiteral, cb), native.runtimePreparedTake, eventError);
    await operation(cb => native.runtimeSnapshotExecute(snapshot, badLiteral, [], cb), native.runtimeResultTake, eventError);
    const badScalar = query(variable, [{ kind: 'compute', expr: { kind: 'literal', value: tagged(malformed) } }]);
    await operation(cb => native.runtimeSnapshotPrepare(snapshot, badScalar, cb), native.runtimePreparedTake, eventError);
    const badComparison = query(variable, [{ kind: 'count' }], [{ kind: 'leaf', cmp: { op: { kind: 'eq' }, lhs: variable, rhs: { kind: 'literal', value: tagged(malformed) } } }]);
    await operation(cb => native.runtimeSnapshotPrepare(snapshot, badComparison, cb), native.runtimePreparedTake, eventError);
    const badImport = query(variable, [{ kind: 'event', expr: { kind: 'map', op: 'image', descriptor: Buffer.from('BEDC'), expr: variable } }]);
    await operation(cb => native.runtimeSnapshotPrepare(snapshot, badImport, cb), native.runtimePreparedTake, eventError);
    const badFaces = query(variable, [{ kind: 'event', expr: { kind: 'relation', op: 'region', relation: { kind: 'bind', descriptor: Buffer.from('BEDC'), expr: variable } } }]);
    await operation(cb => native.runtimeSnapshotPrepare(snapshot, badFaces, cb), native.runtimePreparedTake, eventError);
    // Intrinsic backing checks run before copying and cannot be bypassed by
    // shadowing .buffer. Neither an ArrayBuffer view nor its bytes are retained.
    const shared = new Uint8Array(new SharedArrayBuffer(valid.length));
    shared.set(valid);
    Object.defineProperty(shared, 'buffer', { value: new ArrayBuffer(valid.length) });
    const unexpected = () => assert.fail('shared input was scheduled');
    for (const start of [
      () => native.runtimeEncodeRows(runtime, spec, 0, 1n, [shared], unexpected),
      () => native.runtimeSnapshotGet(snapshot, 0, 0, [shared], unexpected),
      () => native.runtimePreparedExecute(prepared, [tagged(shared)], unexpected),
      () => native.runtimeSnapshotPrepare(snapshot, query({ kind: 'literal', value: tagged(shared) }), unexpected),
      () => native.runtimeSnapshotPrepare(snapshot, query(variable, [{ kind: 'event', expr: { kind: 'map', op: 'image', descriptor: shared, expr: variable } }]), unexpected)
    ]) assert.throws(start, { _tag: 'InvalidArgument' });
    const detached = Uint8Array.from(valid);
    structuredClone(detached, { transfer: [detached.buffer] });
    assert.throws(() => native.runtimePreparedExecute(prepared, [tagged(detached)], unexpected), { _tag: 'InvalidArgument' });
    // A successful owned copy survives mutation immediately after submission.
    const input = Uint8Array.from(valid);
    const encoded = await operation(cb => {
      const handle = native.runtimeEncodeRows(runtime, spec, 0, 1n, [input], cb);
      input.fill(0);
      return handle;
    }, native.runtimeBytesTake);
    const decoded = await operation(cb => native.runtimeDecodeRows(runtime, spec, 0, encoded, cb), native.runtimeRowsTake);
    assert.deepEqual(Buffer.from(decoded[0][0]), valid);
    const emptyResult = await operation(cb => native.runtimePreparedExecute(prepared, [tagged(valid)], cb), native.runtimeResultTake);
    await close(native.runtimeResultClose, emptyResult);
    assert.equal(native.runtimeInspect(runtime).retained, baseline);
    console.log(JSON.stringify({ passed: true, deferred_semantic_refusals: deferred,
      addon_sha256: createHash('sha256').update(fs.readFileSync(binary)).digest('hex') }));
  } finally {
    if (prepared) await close(native.runtimePreparedClose, prepared);
    for (const draft of drafts) await close(native.runtimeDraftClose, draft);
    if (snapshot) await close(native.runtimeSnapshotClose, snapshot);
    if (db) await close(native.runtimeManagedDbClose, db);
    if (directory) await new Promise(resolve => native.runtimeDirectoryClose(directory, false, resolve));
    await close(native.runtimeClose, runtime);
  }
}
main().finally(() => fs.rmSync(temp, { recursive: true, force: true })).catch(error => { console.error(error); process.exitCode = 1; });
