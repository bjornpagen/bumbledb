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
    // The pinned canonical row fixture contains one coordinate predicate.
    // Complement toggles its root; terminal full/empty need no graph nodes.
    const fixture = Buffer.from(fs.readFileSync(path.join(__dirname, '../crates/bumbledb/tests/fixtures/event-v1-row.hex'), 'utf8').trim(), 'hex').subarray(11);
    assert.equal(fixture.subarray(0, 4).toString(), 'BEVT');
    const complement = Buffer.from(fixture);
    complement.writeUInt32LE(fixture.readUInt32LE(48) ^ 1, 48);
    assert.equal(fixture[5], 2);
    const secondCoordinate = Buffer.from(fixture); secondCoordinate[52] = 1;
    const full = Buffer.alloc(52); fixture.copy(full, 0, 0, 40);
    full.writeUInt32LE(1, 44); full.writeUInt32LE(1, 48);
    const empty = Buffer.from(full); empty.writeUInt32LE(0, 48);
    const a = { kind: 'var', var: 0 };
    const expressions = [a, { kind: 'empty', var: 0 }, { kind: 'full', var: 0 },
      { kind: 'not', expr: a }, { kind: 'ite', condition: a, high: a, low: a },
      { kind: 'cardinality', minimum: 2n, maximum: 2n, events: [a, a] },
      ...Array.from({ length: 16 }, (_, bits) => ({ kind: 'apply', bits, left: a, right: a }))];
    const number = value => { const bytes = Buffer.alloc(8); bytes.writeBigUInt64LE(BigInt(value)); return bytes; };
    const blob = bytes => Buffer.concat([number(bytes.length), bytes]);
    const mapBytes = (source, target, ...readouts) => Buffer.concat([
      Buffer.from('BEDC'), Buffer.from([1, 0]), blob(source), blob(target),
      number(readouts.length), ...readouts.map(blob)]);
    const inverse = mapBytes(full, full, complement, secondCoordinate);
    const mapOps = ['pullback', 'image', 'universalImage', 'nonvacuousImage', 'possible', 'guaranteed'];
    expressions.push(...mapOps.map(op => ({ kind: 'map', op, descriptor: inverse, expr: a })));
    const bound = { kind: 'bound', depth: 0 };
    const fixed = (op, expr, scope = full) => ({ kind: 'fixed', op, scope, expr });
    expressions.push(fixed('least', bound), fixed('greatest', bound),
      fixed('least', { kind: 'apply', bits: 14, left: a, right: bound }),
      fixed('greatest', fixed('least', { kind: 'apply', bits: 14,
        left: bound, right: { kind: 'bound', depth: 1 } })));
    const valid = expressions.map(expr => ({ kind: 'event', expr }));
    valid.push(...['isEmpty', 'isFull'].map(kind => ({ kind: 'test', expr: { kind, expr: a } })),
      ...['subset', 'equal', 'disjoint', 'covers'].map(kind => ({ kind: 'test', expr: { kind, left: a, right: a } })));
    valid.push({ kind: 'pack', over: 0 });
    const query = find => ({ kind: 'cq', interiors: [], head: [find.kind === 'pack' ? { kind: 'aggregate', op: 'pack' } : { kind: 'compute' }], rules: [{ finds: [find],
      atoms: [{ source: { kind: 'edb', relation: 0 }, bindings: [[0, { kind: 'var', var: 0 }]] }], negated: [], conditions: [] }] });
    for (const find of valid) {
      const prepared = await operation(cb => native.runtimeSnapshotPrepare(snapshot, query(find), cb), native.runtimePreparedTake);
      await close(native.runtimePreparedClose, prepared);
    }
    const malformed = [
      bound, { kind: 'bound', depth: -1 }, { kind: 'bound', depth: 65536 },
      { kind: 'bound', depth: 0, ignored: true }, fixed('wrong', bound),
      fixed('least', { kind: 'bound', depth: 1 }),
      fixed('least', { kind: 'not', expr: bound }),
      fixed('least', fixed('greatest', { kind: 'not', expr: { kind: 'bound', depth: 1 } })),
      fixed('least', bound, fixture), fixed('least', bound, Buffer.from('BEVT')),
      { ...fixed('least', bound), ignored: true },
      fixed('least', bound, new Uint8Array(16 * 1024 * 1024 + 1)),
      { kind: 'full', var: 0, ignored: true }, { kind: 'var', var: -1 },
      { kind: 'map', op: 'image', descriptor: new Uint8Array(), expr: a },
      { kind: 'map', op: 'unknown', descriptor: inverse, expr: a },
      { kind: 'map', op: 'image', descriptor: inverse, expr: a, ignored: true },
      { kind: 'map', op: 'image', descriptor: mapBytes(fixture, full, fixture, secondCoordinate), expr: a },
      { kind: 'map', op: 'image', descriptor: mapBytes(full, full, fixture), expr: a },
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
    const collect = async ir => {
      const result = await operation(cb => native.runtimeSnapshotExecute(snapshot, ir, [], cb), native.runtimeResultTake);
      try { return await operation(cb => native.runtimeResultCollect(result, cb), native.runtimeRowsTake); }
      finally { await close(native.runtimeResultClose, result); }
    };
    const pack = query({ kind: 'pack', over: 0 });
    assert.deepEqual(await collect(pack), []);
    const insert = async values => {
      const draft = await operation(cb => native.runtimeDraftOpen(runtime, spec, cb), native.runtimeDraftTake);
      let changes;
      try {
        await operation(cb => native.runtimeDraftInsert(draft, 0, BigInt(values.length), values, cb), native.runtimeReportTake);
        changes = (await operation(cb => native.runtimeDraftFinish(draft, cb), native.runtimeChangesTake)).changes;
        const applied = await operation(cb => native.runtimeDbApply(db, changes, { kind: 'any' }, cb), native.runtimeApplyTake);
        assert.equal(applied.tag, 'accepted');
      } finally {
        if (changes) await close(native.runtimeChangesClose, changes);
        await close(native.runtimeDraftClose, draft);
      }
      await close(native.runtimeSnapshotClose, snapshot);
      snapshot = (await operation(cb => native.runtimeDbSnapshot(db, cb), native.runtimeSnapshotTake)).snapshot;
    };
    await insert([fixture, complement, fixture]);
    const packed = await collect(pack);
    assert.equal(packed.length, 1);
    assert.deepEqual(Buffer.from(packed[0][0]), full);
    const numberChecks = await require('./event-number-wire.cjs')({ native, runtime, snapshot, operation, close, collect, query });
    const staged = query({ kind: 'event', expr: { kind: 'not', expr: a } });
    staged.interiors = [{ head: pack.head, rules: pack.rules }];
    staged.rules[0].atoms[0].source = { kind: 'interior', interior: 0 };
    const missing = await collect(staged);
    assert.equal(missing.length, 1);
    assert.deepEqual(Buffer.from(missing[0][0]), empty);
    for (const op of mapOps) {
      const ir = query({ kind: 'event', expr: { kind: 'map', op, descriptor: inverse, expr: a } });
      ir.head.unshift({ kind: 'var' });
      ir.rules[0].finds.unshift({ kind: 'var', var: 0 });
      const rows = await collect(ir);
      assert.equal(rows.length, 2);
      for (const [before, after] of rows) {
        const unchanged = ['possible', 'guaranteed'].includes(op);
        const expected = unchanged ? before : (Buffer.from(before).equals(fixture) ? complement : fixture);
        assert.deepEqual(Buffer.from(after), Buffer.from(expected));
      }
    }
    const target = Buffer.from(full); target.fill(44, 8, 40);
    const moved = await collect(query({ kind: 'event', expr: {
      kind: 'map', op: 'image', descriptor: mapBytes(full, target, fixture, secondCoordinate), expr: a } }));
    assert.equal(moved.length, 2);
    for (const [value] of moved) assert.deepEqual(Buffer.from(value).subarray(8, 40), target.subarray(8, 40));
    const relationChecks = await require('./event-relation-wire.cjs')({ collect, query, fixture, full, a, number, blob });
    const measured = Buffer.from(fs.readFileSync(path.join(__dirname, '../crates/bumbledb/tests/fixtures/event-v2-source.hex'), 'utf8').trim(), 'hex');
    const measuredComplement = Buffer.from(measured);
    measuredComplement.writeUInt32LE(measured.readUInt32LE(13 + 48) ^ 1, 13 + 48);
    await insert([measured, measured]);
    const echo = query({ kind: 'var', var: 0 }); echo.head = [{ kind: 'var' }];
    const measuredRows = await collect(echo);
    assert.equal(measuredRows.length, 3);
    assert.equal(measuredRows.filter(([value]) => Buffer.from(value).equals(measured)).length, 1);
    const complementRows = await collect(query({ kind: 'event', expr: { kind: 'not', expr: a } }));
    assert.equal(complementRows.filter(([value]) => Buffer.from(value).equals(measuredComplement)).length, 1);
    const badLaw = Buffer.from(measured);
    const rational = badLaw.indexOf(Buffer.from('BERA'));
    badLaw[rational + 23] = 3;
    await assert.rejects(() => insert([badLaw]));
    assert.equal((await collect(echo)).length, 3);
    const foreign = Buffer.from(empty); foreign.fill(43, 8, 40);
    await insert([foreign]);
    await assert.rejects(() => collect(pack));
    await assert.rejects(() => collect(query({ kind: 'event', expr: { kind: 'map', op: 'image', descriptor: inverse, expr: a } })));
    console.log(JSON.stringify({ passed: true, constructors_admitted: valid.length,
      malformed_programs_refused: malformed.length,
      event_pack_evaluations: 4,
      event_map_evaluations: 8,
      measured_wire_evaluations: 4, malformed_laws_refused: 1,
      ...relationChecks, ...numberChecks,
      addon_sha256: createHash('sha256').update(fs.readFileSync(binary)).digest('hex') }));
  } finally {
    if (snapshot) await close(native.runtimeSnapshotClose, snapshot);
    if (db) await close(native.runtimeManagedDbClose, db);
    if (directory) await close((handle, callback) => native.runtimeDirectoryClose(handle, false, callback), directory);
    await close(native.runtimeClose, runtime);
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => fs.rmSync(temp, { recursive: true, force: true }));
