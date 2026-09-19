// Raw grammar and owned transport, without the TypeScript builder's checks.
module.exports = async ({ native, runtime, snapshot, operation, close, collect, query }) => {
  const assert = require('node:assert/strict');
  const exact = async n => operation(cb => native.runtimeExactRational(runtime, 'fraction',
    [Buffer.from(String(n)), Buffer.from('1')], 0, cb), native.runtimeBytesTake);
  const one = { kind: 'literal', bytes: await exact(1) };
  const two = { kind: 'literal', bytes: await exact(2) };
  const expr = { kind: 'divide', left: one, right: two };
  const rows = await collect(query({ kind: 'number', expr }));
  assert.equal(rows.length, 1);
  const answer = rows[0][0];
  assert.equal(answer.kind, 'number');
  assert.equal(answer.law, 'fixed');
  assert.equal(Buffer.from(answer.bytes).subarray(0, 5).toString(), 'BENO\x01');
  assert.equal(Buffer.from(answer.value).subarray(0, 5).toString(), 'BERA\x01');
  assert.equal(answer.defined, null); assert.equal(answer.domain, null);
  const imported = await collect(query({ kind: 'number', expr: { kind: 'imported', bytes: answer.bytes } }));
  assert.deepEqual(imported, rows);
  const malformed = [
    { kind: 'integer', var: 0 }, // the only stored variable is Event
    { kind: 'var', var: 0 },
    { kind: 'component', observation: 0, component: 'value' },
    { kind: 'component', observation: 0, component: 'unknown' },
    { kind: 'literal', bytes: Buffer.from('BERA\x02') },
    { kind: 'imported', bytes: Buffer.from('BENO\x01') },
    { kind: 'pow', value: one, exponent: -1 },
    { kind: 'pow', value: one, exponent: 0x100000000 },
    { kind: 'pow', value: one, exponent: 0.5 },
    { kind: 'onDomain', value: one, domain: Buffer.from('BEPR\x01') },
    { ...one, ignored: true },
    { kind: 'add', left: one },
    { kind: 'imported', bytes: new Uint8Array(new SharedArrayBuffer(5)) },
  ];
  const cycle = { kind: 'negate' }; cycle.value = cycle; malformed.push(cycle);
  for (const expr of malformed) {
    await assert.rejects(() => operation(cb => native.runtimeSnapshotPrepare(snapshot,
      query({ kind: 'number', expr }), cb), native.runtimePreparedTake));
  }
  const prepared = await operation(cb => native.runtimeSnapshotPrepare(snapshot,
    query({ kind: 'number', expr }), cb), native.runtimePreparedTake);
  await close(native.runtimePreparedClose, prepared);
  return { numerical_wire_evaluations: 2, malformed_numerical_programs_refused: malformed.length };
};
