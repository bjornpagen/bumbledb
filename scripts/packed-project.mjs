import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/** Bare consumers retain the release's exact package-manager and peer pins. */
export function consumerManifest(core, log, pure = false) {
  assert.match(core.packageManager, /^pnpm@\d+\.\d+\.\d+$/, 'pin an exact pnpm version');
  assert.equal(log.packageManager, core.packageManager, 'SDK package-manager pins disagree');
  assert.equal(log.version, core.version, 'SDK versions disagree');
  assert.equal(log.peerDependencies.effect, core.peerDependencies.effect, 'Effect peers disagree');
  return {
    name: pure ? 'packed-pure-authoring' : 'packed-import-consumer',
    private: true,
    type: 'module',
    packageManager: core.packageManager,
    dependencies: {
      '@bjornpagen/bumbledb': `file:../bjornpagen-bumbledb-${core.version}.tgz`,
      ...(!pure ? { '@bjornpagen/bumbledb-log': `file:../bjornpagen-bumbledb-log-${core.version}.tgz` } : {}),
      effect: core.peerDependencies.effect,
    },
    devDependencies: {
      '@types/node': log.devDependencies['@types/node'],
      typescript: log.devDependencies.typescript,
    },
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [root, output, mode] = process.argv.slice(2);
  assert(root && output && (mode === undefined || mode === '--pure'), 'usage: packed-project.mjs ROOT OUTPUT [--pure]');
  const read = name => JSON.parse(fs.readFileSync(path.join(root, name, 'package.json'), 'utf8'));
  const manifest = consumerManifest(read('ts'), read('ts-log'), mode === '--pure');
  fs.writeFileSync(path.join(output, 'package.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
}
