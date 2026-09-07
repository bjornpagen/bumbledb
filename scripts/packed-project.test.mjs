import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import { consumerManifest } from './packed-project.mjs';

const core = JSON.parse(fs.readFileSync(new URL('../ts/package.json', import.meta.url), 'utf8'));
const log = JSON.parse(fs.readFileSync(new URL('../ts-log/package.json', import.meta.url), 'utf8'));

for (const pure of [false, true]) {
  test(`isolated ${pure ? 'pure' : 'native'} consumer retains exact toolchain and release pins`, () => {
    const manifest = consumerManifest(core, log, pure);
    assert.equal(manifest.packageManager, core.packageManager);
    assert.equal(manifest.dependencies.effect, core.peerDependencies.effect);
    assert.equal(manifest.dependencies['@bjornpagen/bumbledb'], `file:../bjornpagen-bumbledb-${core.version}.tgz`);
    assert.equal(manifest.dependencies['@bjornpagen/bumbledb-log'], pure ? undefined : `file:../bjornpagen-bumbledb-log-${core.version}.tgz`);
    assert.equal(manifest.devDependencies.typescript, log.devDependencies.typescript);
    assert.equal(manifest.scripts, undefined, 'consumer must not enable lifecycle scripts');
  });
}

test('unversioned or inconsistent package managers fail before writing a consumer', () => {
  for (const packageManager of [undefined, 'pnpm', 'pnpm@latest', 'pnpm@^11.9.0', 'npm@11.9.0']) {
    assert.throws(() => consumerManifest({ ...core, packageManager }, log));
  }
  assert.throws(() => consumerManifest(core, { ...log, packageManager: 'pnpm@12.3.4' }));
});

test('version and Effect drift cannot leak into isolated consumers', () => {
  assert.throws(() => consumerManifest(core, { ...log, version: '0.0.0' }));
  assert.throws(() => consumerManifest(core, { ...log, peerDependencies: { ...log.peerDependencies, effect: '0.0.0' } }));
});
