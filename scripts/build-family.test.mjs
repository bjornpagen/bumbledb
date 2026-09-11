import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { test } from "node:test"
import { buildFamily, checkBuildSource, snapshotSource } from "./build-family.mjs"

function fixture(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-family-test-"))
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))
  const root = path.join(dir, "repo")
  fs.mkdirSync(root)
  const git = (...args) => execFileSync("git", ["-C", root, ...args], { encoding: "utf8" })
  git("init", "-q")
  fs.writeFileSync(path.join(root, ".gitignore"), "dist/\n")
  fs.writeFileSync(path.join(root, "input.ts"), "export const input = 1\n")
  fs.writeFileSync(path.join(root, "removed.ts"), "export const removed = 1\n")
  git("add", ".")
  git("-c", "user.name=test", "-c", "user.email=test@localhost", "commit", "-qm", "fixture")
  fs.mkdirSync(path.join(root, "dist"))
  fs.writeFileSync(path.join(root, "dist", "live.js"), "export default 1\n")
  return { dir, root, git }
}

test("selection includes staged, unstaged, new and deleted inputs without changing the user's index", t => {
  const { root, dir, git } = fixture(t)
  fs.writeFileSync(path.join(root, "input.ts"), "export const input = 2\n")
  git("add", "input.ts")
  fs.writeFileSync(path.join(root, "input.ts"), "export const input = 3\n")
  fs.writeFileSync(path.join(root, "new.ts"), "export const added = 4\n")
  fs.unlinkSync(path.join(root, "removed.ts"))
  const before = git("diff", "--cached")
  const source = path.join(dir, "selected")
  const selected = snapshotSource(root, source)
  assert.equal(git("diff", "--cached"), before)
  assert.equal(fs.readFileSync(path.join(source, "input.ts"), "utf8"), "export const input = 3\n")
  assert.equal(fs.readFileSync(path.join(source, "new.ts"), "utf8"), "export const added = 4\n")
  assert.equal(fs.existsSync(path.join(source, "removed.ts")), false)
  assert.equal(fs.existsSync(path.join(source, "dist")), false)
  fs.writeFileSync(path.join(root, "input.ts"), "export const input = 5\n")
  assert.deepEqual(checkBuildSource(source), selected)
  fs.writeFileSync(path.join(source, "input.ts"), "export const input = 6\n")
  assert.throws(() => checkBuildSource(source))
})

test("overlapping selections remain independent across source edits and failed attempts", t => {
  const { root, dir } = fixture(t)
  const first = path.join(dir, "first")
  const second = path.join(dir, "second")
  const a = snapshotSource(root, first)
  fs.writeFileSync(path.join(root, "input.ts"), "export const input = 2\n")
  const b = snapshotSource(root, second)
  assert.notEqual(a.tree, b.tree)
  fs.mkdirSync(path.join(first, "dist"))
  fs.writeFileSync(path.join(first, "dist", "retained.js"), "retained")
  const out = path.join(dir, "failed")
  assert.throws(() => buildFamily({ sourceRoot: root, out, build: source => {
    assert.equal(fs.readFileSync(path.join(source, "input.ts"), "utf8"), "export const input = 2\n")
    throw Error("build failure")
  } }), /build failure/)
  assert.equal(fs.existsSync(out), false)
  assert.equal(fs.readdirSync(dir).some(name => name.startsWith(".bumbledb-build-")), false)
  assert.equal(fs.readFileSync(path.join(first, "dist", "retained.js"), "utf8"), "retained")
  assert.deepEqual(checkBuildSource(first), a)
  assert.deepEqual(checkBuildSource(second), b)
  assert.equal(fs.readFileSync(path.join(root, "dist", "live.js"), "utf8"), "export default 1\n")
})

test("mutable external symlinks cannot become implicit build inputs", t => {
  const { root, dir } = fixture(t)
  const external = path.join(dir, "external.ts")
  fs.writeFileSync(external, "export const external = 1\n")
  fs.symlinkSync(external, path.join(root, "external.ts"))
  assert.throws(() => snapshotSource(root, path.join(dir, "selected")), /escapes the selected source/)
})

test("release CLIs execute when their absolute entry paths cross a symlink", t => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cli-link-"))
  t.after(() => fs.rmSync(base, {recursive:true,force:true}))
  for (const [name, reason] of [
    ["build-family.mjs", /unknown build argument/],
    ["release-results.mjs", /Usage:/],
    ["release-ready.mjs", /usage:/],
    ["packed-project.mjs", /usage:/],
  ]) {
    const entry = path.join(base, name)
    fs.symlinkSync(path.join(import.meta.dirname, name), entry)
    assert.throws(() => execFileSync(process.execPath, [entry, "--unknown"], {stdio:"pipe"}), error => {
      assert.match(error.stderr.toString(), reason)
      return true
    })
  }
})
