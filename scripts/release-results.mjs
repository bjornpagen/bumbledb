// Checks release evidence; never runs tests, manufactures results, or publishes.
import fs from "node:fs"
import path from "node:path"
import crypto from "node:crypto"
import { execFileSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const read = name => fs.readFileSync(path.join(root, name), "utf8")

export function inventory() {
  const audit = [...read("final-solution/50-audit-closure-matrix.md").matchAll(/^\| ((?:REP|ENG|QRY|SDK|ARCH|OPS|PERF|ASS)-\d+) \|/gm)].map(match => match[1])
  const children = []
  for (const line of read("final-solution/70-test-and-release-gates.md").split("\n")) {
    if (!line.startsWith("| [")) continue
    for (const match of line.split("|")[2].matchAll(/`([A-Z0-9-]+)`(?: through `([A-Z0-9-]+)`)?/g)) {
      if (!match[2]) { children.push(match[1]); continue }
      const first = /^(.*-)(\d+)$/.exec(match[1]), last = /^(.*-)(\d+)$/.exec(match[2])
      if (!first || !last || first[1] !== last[1] || +first[2] > +last[2]) throw Error(`Invalid gate range: ${match[0]}`)
      for (let n = +first[2]; n <= +last[2]; n++) children.push(first[1] + String(n).padStart(first[2].length, "0"))
    }
  }
  if (!audit.length || !children.length || new Set(children).size !== children.length) throw Error("Empty/duplicate specification inventory")
  for (const file of fs.readdirSync(path.join(root, 'final-solution')).filter(file => file.endsWith('.md') && !/^(50|70)/.test(file))) {
    for (const line of read(`final-solution/${file}`).split('\n')) {
      const match = /^\|\s*`?([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)`?(?:\s|\|)/.exec(line)
      if (match && !/^(REP|ENG|QRY|SDK|ARCH|OPS|PERF|ASS)-\d/.test(match[1]) && !children.includes(match[1])) throw Error(`Chapter gate missing from release index: ${match[1]}`)
    }
  }
  return { audit, gates: [...Array.from({length:17}, (_, i) => `G${String(i).padStart(2,"0")}`), ...children] }
}

export function validateResults(manifest, expected, revision, phase, verifyFile) {
  const errors = []
  if (manifest.format !== 1) errors.push("unsupported result format")
  if (manifest.sourceRevision !== revision) errors.push("result source revision is missing or stale")
  if (!/^[a-f0-9]{40}$/.test(manifest.specificationRevision ?? "")) errors.push("missing specification revision")
  for (const [key, ids] of [["audits", expected.audit], ["gates", expected.gates]]) {
    const rows = manifest[key]
    if (!Array.isArray(rows)) { errors.push(`missing ${key}`); continue }
    const found = new Set()
    for (const row of rows) {
      if (found.has(row.id)) errors.push(`duplicate ${row.id}`)
      found.add(row.id)
      if (!ids.includes(row.id)) errors.push(`unknown ${row.id}`)
      if (!['Passed','Failed','NotRun','NotApplicable'].includes(row.outcome)) errors.push(`invalid outcome ${row.id}`)
      if (row.id === "PKG-07B" && phase === "pre-promotion") continue
      if (row.outcome !== "Passed") { errors.push(`${row.id}: ${row.outcome} (required)`); continue }
      if (!Array.isArray(row.evidence) || !row.evidence.length) { errors.push(`${row.id}: no evidence`); continue }
      for (const evidence of row.evidence) {
        const prefix = `${row.id}: `
        if (evidence.sourceRevision !== revision || evidence.specificationRevision !== manifest.specificationRevision) errors.push(prefix + "stale evidence revision")
        if (!Number.isSafeInteger(evidence.executed) || evidence.executed < 1 || !Array.isArray(evidence.tests) || !evidence.tests.length) errors.push(prefix + "empty executed test inventory")
        if (!Number.isSafeInteger(evidence.skipped) || evidence.skipped < 0 || (evidence.skipped > 0 && !evidence.skipReasons?.length)) errors.push(prefix + "unexplained skipped cases")
        for (const field of ['platform','toolchain','features','command','review']) if (typeof evidence[field] !== 'string' || !evidence[field]) errors.push(prefix + `missing ${field}`)
        for (const field of ['artifact','report']) {
          const file = evidence[field]
          if (!file || !/^[a-f0-9]{64}$/.test(file.sha256 ?? "") || !file.path) errors.push(prefix + `missing ${field} hash/path`)
          else if (!verifyFile(file)) errors.push(prefix + `${field} missing or hash mismatch`)
        }
      }
    }
    for (const id of ids) if (!found.has(id)) errors.push(`missing ${id}`)
  }
  return errors
}

function main() {
  const args = process.argv.slice(2)
  const expected = inventory()
  if (args.length === 1 && args[0] === "--inventory") {
    process.stdout.write(`${expected.audit.length} audit issues; ${expected.gates.length - 17} child families; 17 parent gates\n`)
    return
  }
  const [phase, file, revisionOverride] = args
  if (!['pre-promotion','post-promotion'].includes(phase) || args.length > 3) throw Error("Usage: node scripts/release-results.mjs --inventory | pre-promotion|post-promotion [manifest.json] [exact-staged-revision]")
  const revision = revisionOverride ?? execFileSync("git", ['rev-parse','HEAD'], {cwd:root,encoding:'utf8'}).trim()
  if (!/^[a-f0-9]{40}$/.test(revision)) throw Error("An exact candidate revision is required")
  const manifest = JSON.parse(read(file ?? 'implementation/release-results.json'))
  const errors = validateResults(manifest, expected, revision, phase, file => {
    try { return crypto.createHash('sha256').update(fs.readFileSync(path.resolve(root,file.path))).digest('hex') === file.sha256 } catch { return false }
  })
  if (errors.length) {
    process.stderr.write(`Release NOT qualified: ${errors.length} unresolved checks.\n${errors.slice(0,12).join('\n')}\n`)
    process.exitCode = 1
  } else process.stdout.write(`Release evidence complete for ${revision} (${phase}); no publication performed.\n`)
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
