#!/usr/bin/env node
// Compile private core/log SDK copies against one private addon. The live
// checkout and loaded addon are never replaced; this is not a release build.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = process.argv[2];
if (!binary) throw new Error('usage: check-event-log-sdk.mjs <compiled addon> [test names...]');
const temp = mkdtempSync(path.join(tmpdir(), 'bumbledb-event-log-sdk-'));
function run(cwd, command, args) {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit', timeout: 300_000 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.signal})`);
}
try {
  const modules = path.join(temp, 'node_modules');
  mkdirSync(modules);
  for (const name of ['ts', 'ts-log']) {
    const source = path.join(root, name);
    const target = path.join(temp, name);
    mkdirSync(target);
    for (const item of ['src', 'test', 'scripts', 'package.json', 'tsconfig.json', 'tsconfig.build.json', 'README.md']) {
      if (existsSync(path.join(source, item))) cpSync(path.join(source, item), path.join(target, item), { recursive: true });
    }
    // Shared dependency identity matters for Effect services across packages.
    const dependencies = path.join(source, 'node_modules');
    for (const entry of existsSync(dependencies) ? readdirSync(dependencies) : []) {
      if (entry.startsWith('.') || entry === '@bjornpagen' || existsSync(path.join(modules, entry))) continue;
      symlinkSync(realpathSync(path.join(source, 'node_modules', entry)), path.join(modules, entry), 'dir');
    }
  }
  const scope = path.join(modules, '@bjornpagen');
  mkdirSync(scope);
  symlinkSync(path.join(temp, 'ts'), path.join(scope, 'bumbledb'), 'dir');
  symlinkSync(path.join(temp, 'ts-log'), path.join(scope, 'bumbledb-log'), 'dir');
  const platform = `bumbledb-${process.platform}-${process.arch}`;
  const native = path.join(scope, platform);
  mkdirSync(native);
  writeFileSync(path.join(native, 'package.json'), JSON.stringify({ name: `@bjornpagen/${platform}`, main: 'bumbledb.node' }));
  cpSync(path.resolve(binary), path.join(native, 'bumbledb.node'));
  const tsc = path.join(root, 'ts/node_modules/.bin/tsc');
  for (const name of ['ts', 'ts-log']) {
    const cwd = path.join(temp, name);
    run(cwd, tsc, ['-p', 'tsconfig.build.json']);
    run(cwd, process.execPath, ['--input-type=module', '-e',
      'import { rewriteDeclarationImports, assertDeclarationsAreIsolated } from "./scripts/declarations.ts"; rewriteDeclarationImports("./dist"); assertDeclarationsAreIsolated("./dist");']);
  }
  const log = path.join(temp, 'ts-log');
  run(log, tsc, ['--noEmit']);
  const requested = process.argv.slice(3);
  const tests = requested.length ? requested : ['event-selections', 'real-bridge-roundtrip', 'transition', 'command-boundaries', 'history-open', 'admin', 'borrows', 'adversarial-capabilities', 'submit-certainty', 'outcome-certainty'];
  const names = tests.map(name => {
    if (!/^[a-z0-9-]+$/.test(name)) throw new Error(`invalid test name ${name}`);
    return `test/${name}.test.ts`;
  });
  run(log, process.execPath, ['--test', ...names]);
  console.log(JSON.stringify({ node: process.version, platform, tests,
    version: JSON.parse(readFileSync(path.join(log, 'package.json'), 'utf8')).version,
    addon_sha256: createHash('sha256').update(readFileSync(path.join(native, 'bumbledb.node'))).digest('hex') }));
} finally {
  rmSync(temp, { recursive: true, force: true });
}
