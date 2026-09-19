// Exercise native predicate ingress independently of the SDK authoring checks.
module.exports = async ({ native, runtime, snapshot, operation, close, collect, query }) => {
  const assert = require('node:assert/strict');
  const exact = async n => operation(cb => native.runtimeExactRational(runtime, 'fraction',
    [Buffer.from(String(n)), Buffer.from('1')], 0, cb), native.runtimeBytesTake);
  const one = { kind: 'literal', bytes: await exact(1) };
  const zero = { kind: 'literal', bytes: await exact(0) };
  const truth = { kind: 'sign', number: one, signs: 4 };
  const hole = { kind: 'sign', number: { kind: 'divide', left: one, right: zero }, signs: 7 };
  let evaluations = 0;
  for (const [expr, expected] of [
    [truth, true], [{ kind: 'negate', value: truth }, false], [hole, null],
    [{ kind: 'apply', op: 15, left: truth, right: hole }, null],
  ]) {
    const rows = await collect(query({ kind: 'predicate', expr }));
    assert.equal(rows.length, 1);
    const answer = rows[0][0];
    assert.equal(answer.kind, 'predicate'); assert.equal(answer.law, 'fixed');
    assert.equal(answer.value, expected);
    for (const field of ['holds','fails','undefined','domain']) assert.equal(answer[field], null);
    assert.equal(Buffer.from(answer.bytes).subarray(0,5).toString(), 'BENP\x01');
    assert.deepEqual(await collect(query({ kind: 'predicate', expr: { kind: 'imported', bytes: answer.bytes } })), rows);
    for (const quantifier of ['possibly', 'always', 'isTotal']) {
      const result = await collect(query({ kind: 'predicateTest', expr, quantifier }));
      assert.deepEqual(result, [[quantifier === 'isTotal' ? expected !== null : expected === true]]);
      evaluations++;
    }
    evaluations += 2;
  }
  const parameter = async (op, inputs) => operation(cb => native.runtimeEventParameter(runtime, op, inputs, 0n, cb), native.runtimeBytesTake);
  const name = Buffer.alloc(32, 246);
  const domain = await parameter('region.full', [name]);
  const emptyRegion = await parameter('region.empty', [name]);
  const region = {kind:'region', domain, region:emptyRegion};
  const regionRows = await collect(query({kind:'predicate',expr:region}));
  assert.equal(regionRows[0][0].law,'parameter');
  assert.equal(Buffer.from(regionRows[0][0].bytes).subarray(0,5).toString(),'BENP\x02');
  assert.deepEqual(Buffer.from(regionRows[0][0].holds),Buffer.from(emptyRegion));
  assert.deepEqual(Buffer.from(regionRows[0][0].fails),Buffer.from(domain));
  assert.deepEqual(await collect(query({kind:'predicateTest',expr:region,quantifier:'isTotal'})),[[true]]);
  assert.deepEqual(await collect(query({kind:'predicate',expr:{kind:'imported',bytes:regionRows[0][0].bytes}})),regionRows);
  evaluations+=3;
  const malformed = [
    {...region, ignored:true}, {kind:'region',domain},
    {...region,region:Buffer.from('BEPR\x01')}, {...region,domain:emptyRegion},
    {...region,region:new Uint8Array(new SharedArrayBuffer(5))},
    { kind: 'var', var: 0 }, // an Event variable cannot become a predicate
    { kind: 'sign', number: { kind: 'var', var: 0 }, signs: 7 },
    { ...truth, signs: -1 }, { ...truth, signs: 8 }, { ...truth, signs: 0.5 },
    { kind: 'apply', op: 16, left: truth, right: truth },
    { kind: 'imported', bytes: Buffer.from('BENP\x01') },
    { kind: 'imported', bytes: Buffer.from('BENO\x01') },
    { kind: 'onDomain', value: truth, domain: Buffer.from('BEPR\x01') },
    { ...truth, ignored: true }, { kind: 'apply', op: 15, left: truth },
    { kind: 'imported', bytes: new Uint8Array(new SharedArrayBuffer(5)) },
  ];
  const cycle = { kind: 'negate' }; cycle.value = cycle; malformed.push(cycle);
  let deep = one;
  for (let i = 1; i < 128; i++) deep = { kind: 'abs', value: deep };
  malformed.push({ kind: 'sign', number: deep, signs: 7 });
  let wide = one;
  for (let i = 0; i < 11; i++) wide = { kind: 'add', left: wide, right: wide };
  malformed.push({ kind: 'negate', value: { kind: 'sign', number: wide, signs: 7 } });
  for (const expr of malformed) {
    await assert.rejects(() => operation(cb => native.runtimeSnapshotPrepare(snapshot,
      query({ kind: 'predicate', expr }), cb), native.runtimePreparedTake));
  }
  for (const find of [
    { kind: 'predicateTest', expr: truth, quantifier: 'maybe' },
    { kind: 'predicateTest', expr: truth, quantifier: 'always', ignored: true },
  ]) await assert.rejects(() => operation(cb => native.runtimeSnapshotPrepare(snapshot,
    query(find), cb), native.runtimePreparedTake));
  const prepared = await operation(cb => native.runtimeSnapshotPrepare(snapshot,
    query({ kind: 'predicate', expr: truth }), cb), native.runtimePreparedTake);
  await close(native.runtimePreparedClose, prepared);
  return { predicate_wire_evaluations: evaluations, malformed_predicate_programs_refused: malformed.length + 2 };
};
