#!/usr/bin/env node
// Exercise the SDK against a private addon copy. No live addon or checkout
// build output is replaced. This is integration evidence, not a package build.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = process.argv[2];
if (!binary) throw new Error('usage: check-event-sdk.mjs <compiled addon> [test names...]');
const source = path.join(root, 'ts');
const temp = mkdtempSync(path.join(tmpdir(), 'bumbledb-event-sdk-'));
function run(command, args) {
  const result = spawnSync(command, args, { cwd: temp, stdio: 'inherit', timeout: 300_000 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.signal})`);
}
try {
  for (const name of ['src', 'test', 'scripts', 'package.json', 'tsconfig.json', 'tsconfig.build.json']) {
    cpSync(path.join(source, name), path.join(temp, name), { recursive: true });
  }
  const modules = path.join(temp, 'node_modules');
  mkdirSync(modules);
  for (const entry of readdirSync(path.join(source, 'node_modules'))) {
    if (entry.startsWith('.') || entry === '@bjornpagen') continue;
    symlinkSync(realpathSync(path.join(source, 'node_modules', entry)), path.join(modules, entry), 'dir');
  }
  // The generated-binding compiler uses the installed binaries directly.
  symlinkSync(path.join(source, 'node_modules/.bin'), path.join(modules, '.bin'), 'dir');
  const platform = `@bjornpagen/bumbledb-${process.platform}-${process.arch}`;
  const native = path.join(modules, platform);
  mkdirSync(native, { recursive: true });
  writeFileSync(path.join(native, 'package.json'), JSON.stringify({ name: platform, main: 'bumbledb.node' }));
  cpSync(path.resolve(binary), path.join(native, 'bumbledb.node'));
  run(path.join(source, 'node_modules/.bin/tsc'), ['-p', 'tsconfig.build.json']);
  // The real build uses the same declaration rewrite: generated bindings must
  // resolve their public types without depending on private # import aliases.
  run(process.execPath, ['--input-type=module', '-e',
    'import { rewriteDeclarationImports, assertDeclarationsAreIsolated } from "./scripts/declarations.ts"; rewriteDeclarationImports("./dist"); assertDeclarationsAreIsolated("./dist");']);
  const requested = process.argv.slice(3);
  const tests = requested.length ? requested : ['event', 'event-query', 'event-descriptor', 'event-source', 'schema-bindings', 'pure-import',
    'field-codec', 'query-description', 'comparison-pairing', 'alternatives', 'rows',
    'boundary-codec', 'value-conformance', 'interval-outputs', 'effect-core', 'ownership-interrupt'];
  const names = tests.map(name => {
    if (!/^[a-z0-9-]+$/.test(name)) throw new Error(`invalid test name ${name}`);
    return `test/${name}.test.ts`;
  });
  run(process.execPath, ['--conditions=bumbledb-src', '--test', ...names]);
  const manifest = JSON.parse(readFileSync(path.join(temp, 'package.json'), 'utf8'));
  console.log(`SDK integration passed against isolated ${platform}; version unchanged at ${manifest.version}`);
  console.log(JSON.stringify({ node: process.version, platform, tests,
    addon_sha256: createHash('sha256').update(readFileSync(path.join(native, 'bumbledb.node'))).digest('hex') }));
} finally {
  rmSync(temp, { recursive: true, force: true });
}
