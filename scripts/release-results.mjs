// Checks release evidence; never runs tests, manufactures results, or publishes.
import fs from "node:fs"
import path from "node:path"
import crypto from "node:crypto"
import { execFileSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const read = name => fs.readFileSync(path.join(root, name), "utf8")

const INVENTORY_PATH = "docs/reference/obligation-inventory.json"
const RESULTS_PATH = "docs/reference/release-results.json"

const CANDIDATE_EXCLUDES = [
  /^docs\/reference\/release-results\.json$/,
  /^target\//,
  /^ts\/dist\//,
  /^ts\/node_modules\//,
  /^ts-log\/node_modules\//,
  /^node_modules\//,
  /^\.git\//,
  /^final-solution\/STATUS\.md$/,
]

function uniqueNamed(label, ids) {
  if (!Array.isArray(ids) || !ids.length) throw Error(`inventory missing ${label}`)
  if (new Set(ids).size !== ids.length) throw Error(`Duplicate ${label} inventory`)
  return ids
}

export function loadInventoryDocument() {
  const raw = read(INVENTORY_PATH)
  const doc = JSON.parse(raw)
  if (doc.format !== 1) throw Error(`unsupported inventory format: ${doc.format}`)
  if (!Array.isArray(doc.audit) || !Array.isArray(doc.parentGates) || !Array.isArray(doc.childFamilies)) {
    throw Error("inventory missing audit/parentGates/childFamilies")
  }
  if ((doc.generatedFrom ?? []).some(entry => String(entry).startsWith("final-solution/"))) {
    throw Error("inventory generatedFrom still depends on disposable final-solution Markdown")
  }
  return doc
}

export function inventoryDigest(doc = loadInventoryDocument()) {
  const canonical = JSON.stringify({
    format: doc.format,
    audit: doc.audit,
    parentGates: doc.parentGates,
    priorReview: doc.priorReview ?? [],
    discriminators: doc.discriminators ?? [],
    childFamilies: doc.childFamilies,
    qualificationCells: doc.qualificationCells ?? [],
  })
  return crypto.createHash("sha256").update(canonical).digest("hex")
}

export function inventory() {
  const doc = loadInventoryDocument()
  const audit = uniqueNamed("audit", doc.audit)
  const parentGates = uniqueNamed("parentGates", doc.parentGates)
  const childFamilies = uniqueNamed("childFamilies", doc.childFamilies)
  const priorReview = uniqueNamed("priorReview", doc.priorReview)
  const discriminators = uniqueNamed("discriminators", doc.discriminators)
  const gates = [...parentGates, ...childFamilies]
  if (new Set(gates).size !== gates.length) throw Error("Empty/duplicate specification inventory")
  return {
    audit,
    gates,
    priorReview,
    discriminators,
    qualificationCells: doc.qualificationCells ?? [],
    specificationRevision: inventoryDigest(doc),
  }
}

export function listCandidateSourcePaths() {
  const tracked = execFileSync("git", ["ls-files", "-z"], { cwd: root, encoding: "buffer" })
    .toString("utf8")
    .split("\0")
    .filter(Boolean)
  const untracked = execFileSync("git", ["ls-files", "-z", "--others", "--exclude-standard"], { cwd: root, encoding: "buffer" })
    .toString("utf8")
    .split("\0")
    .filter(Boolean)
  const paths = [...new Set([...tracked, ...untracked])]
    .filter(p => !CANDIDATE_EXCLUDES.some(re => re.test(p)))
    .sort()
  if (!paths.length) throw Error("candidate source inventory is empty")
  return paths
}

/** One deterministic candidate-input row: path, kind, mode, and bytes or symlink target. */
export function describeCandidateEntry(rel) {
  const abs = path.join(root, rel)
  let st
  try {
    st = fs.lstatSync(abs)
  } catch (err) {
    if (err && typeof err === "object" && "code" in err && err.code === "ENOENT") {
      return { rel, kind: "deleted", mode: 0, payload: "" }
    }
    throw err
  }
  if (st.isDirectory()) return null
  const mode = st.mode & 0o777
  if (st.isSymbolicLink()) {
    return { rel, kind: "symlink", mode, payload: fs.readlinkSync(abs) }
  }
  return {
    rel,
    kind: "file",
    mode,
    payload: crypto.createHash("sha256").update(fs.readFileSync(abs)).digest("hex"),
  }
}

export function frameCandidateEntry(entry) {
  return `${entry.rel}\0${entry.kind}\0${entry.mode.toString(8)}\0${entry.payload}\n`
}

export function computeCandidateSourceDigest(paths) {
  const actual = listCandidateSourcePaths()
  if (paths) {
    if (paths.length !== actual.length || paths.some((p, i) => p !== actual[i])) {
      throw Error("caller-supplied candidate path list cannot override recomputed membership")
    }
  }
  const digest = crypto.createHash("sha256")
  for (const rel of actual) {
    const entry = describeCandidateEntry(rel)
    if (entry) digest.update(frameCandidateEntry(entry))
  }
  return digest.digest("hex")
}

function sha256File(abs) {
  return crypto.createHash("sha256").update(fs.readFileSync(abs)).digest("hex")
}

function localNativeBinaryRel() {
  const os = process.platform === "darwin" ? "darwin" : process.platform === "linux" ? "linux" : null
  const arch = process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : null
  if (!os || !arch) throw Error(`unsupported native provenance host ${process.platform}-${process.arch}`)
  return `ts/npm/${os}-${arch}/bumbledb.node`
}

export function nativeProvenancePath(binaryRel = localNativeBinaryRel()) {
  return path.posix.join(path.posix.dirname(binaryRel), ".native-provenance.json")
}

export function writeNativeProvenance(binaryRel = localNativeBinaryRel()) {
  const abs = path.join(root, binaryRel)
  if (!fs.existsSync(abs)) throw Error(`native artifact missing: ${binaryRel}`)
  const stamp = {
    candidateSourceDigest: computeCandidateSourceDigest(),
    specificationRevision: inventory().specificationRevision,
    artifact: { path: binaryRel, sha256: sha256File(abs) },
    platform: `${process.platform}-${process.arch}`,
  }
  const stampRel = nativeProvenancePath(binaryRel)
  fs.writeFileSync(path.join(root, stampRel), `${JSON.stringify(stamp, null, "\t")}\n`)
  return stamp
}

export function verifyNativeProvenance(binaryRel = localNativeBinaryRel()) {
  const abs = path.join(root, binaryRel)
  const stampRel = nativeProvenancePath(binaryRel)
  const stampAbs = path.join(root, stampRel)
  if (!fs.existsSync(abs)) throw Error(`native artifact missing: ${binaryRel}`)
  if (!fs.existsSync(stampAbs)) throw Error(`native provenance missing: ${stampRel}`)
  const stamp = JSON.parse(fs.readFileSync(stampAbs, "utf8"))
  const expectedDigest = computeCandidateSourceDigest()
  const expectedSpec = inventory().specificationRevision
  if (stamp.candidateSourceDigest !== expectedDigest) {
    throw Error("native provenance candidate digest does not match recomputed inventory")
  }
  if (stamp.specificationRevision !== expectedSpec) {
    throw Error("native provenance specification revision does not match obligation inventory")
  }
  if (stamp.artifact?.path !== binaryRel || stamp.artifact?.sha256 !== sha256File(abs)) {
    throw Error("native provenance artifact hash/path does not match the on-disk binary")
  }
  return stamp
}

function validateEvidenceRow(prefix, evidence, manifest, candidateDigest, verifyFile, errors) {
  if (!evidence || typeof evidence !== "object" || Array.isArray(evidence)) {
    errors.push(prefix + "evidence must be a structured record, not a bare string or array")
    return
  }
  if (evidence.specificationRevision !== manifest.specificationRevision) errors.push(prefix + "stale evidence specification revision")
  if (candidateDigest && evidence.candidateSourceDigest !== manifest.candidateSourceDigest) errors.push(prefix + "stale evidence candidate digest")
  if (evidence.sourceRevision && evidence.sourceRevision !== manifest.sourceRevision) errors.push(prefix + "stale evidence source revision")
  if (!Number.isSafeInteger(evidence.executed) || evidence.executed < 1 || !Array.isArray(evidence.tests) || !evidence.tests.length) {
    errors.push(prefix + "empty executed test inventory")
  }
  if (!Number.isSafeInteger(evidence.skipped) || evidence.skipped < 0 || (evidence.skipped > 0 && !evidence.skipReasons?.length)) {
    errors.push(prefix + "unexplained skipped cases")
  }
  for (const field of ["platform", "backend", "toolchain", "features", "command", "review"]) {
    if (typeof evidence[field] !== "string" || !evidence[field]) errors.push(prefix + `missing ${field}`)
  }
  for (const field of ["artifact", "report"]) {
    const file = evidence[field]
    if (!file || typeof file !== "object" || Array.isArray(file)) {
      errors.push(prefix + `missing ${field} hash/path`)
      continue
    }
    if (!/^[a-f0-9]{64}$/.test(file.sha256 ?? "") || !file.path) errors.push(prefix + `missing ${field} hash/path`)
    else if (!verifyFile(file)) errors.push(prefix + `${field} missing or hash mismatch`)
  }
}

const FORBIDDEN_OVERRIDES = ["digestOverride", "candidatePaths", "pathOverride", "specificationOverride"]

export function validateResults(manifest, expected, phase, verifyFile, options = {}) {
  const errors = []
  if (options.digestOverride || options.candidatePaths || options.pathOverride || options.specificationOverride) {
    throw Error("caller-supplied digest or path overrides cannot omit production inputs")
  }
  const candidateDigest = options.candidateSourceDigest ?? null
  if (manifest.format !== 2) errors.push("unsupported result format")
  for (const key of FORBIDDEN_OVERRIDES) {
    if (Object.hasOwn(manifest, key)) errors.push(`${key} is not a permitted qualification input`)
  }
  if (candidateDigest) {
    if (manifest.candidateSourceDigest !== candidateDigest) errors.push("candidate source digest is missing or stale")
  } else if (!/^[a-f0-9]{64}$/.test(manifest.candidateSourceDigest ?? "")) {
    errors.push("missing candidate source digest")
  }
  if (manifest.specificationRevision !== expected.specificationRevision) errors.push("result specification revision is missing or stale")
  if (manifest.sourceRevision && !/^[a-f0-9]{40}$/.test(manifest.sourceRevision)) errors.push("invalid source revision")
  for (const [key, ids] of [
    ["audits", expected.audit],
    ["gates", expected.gates],
    ["priorReviews", expected.priorReview],
    ["discriminators", expected.discriminators],
  ]) {
    const rows = manifest[key]
    if (!Array.isArray(rows)) { errors.push(`missing ${key}`); continue }
    const found = new Set()
    for (const row of rows) {
      if (found.has(row.id)) errors.push(`duplicate ${row.id}`)
      found.add(row.id)
      if (!ids.includes(row.id)) errors.push(`unknown ${row.id}`)
      if (!["Passed", "Failed", "NotRun", "NotApplicable"].includes(row.outcome)) errors.push(`invalid outcome ${row.id}`)
      if (row.id === "PKG-07B" && phase === "pre-promotion") continue
      if (row.outcome !== "Passed") { errors.push(`${row.id}: ${row.outcome} (required)`); continue }
      if (!Array.isArray(row.evidence) || !row.evidence.length) { errors.push(`${row.id}: no evidence`); continue }
      for (const evidence of row.evidence) {
        validateEvidenceRow(`${row.id}: `, evidence, manifest, candidateDigest, verifyFile, errors)
      }
    }
    for (const id of ids) if (!found.has(id)) errors.push(`missing ${id}`)
  }
  const knownCells = new Set(expected.qualificationCells.map(cell => cell.id))
  const requiredCells = expected.qualificationCells.filter(cell => cell.required)
  const reported = new Map()
  for (const cell of manifest.qualification ?? []) {
    if (reported.has(cell.id)) errors.push(`duplicate ${cell.id}`)
    reported.set(cell.id, cell)
    if (!knownCells.has(cell.id)) errors.push(`unknown ${cell.id}`)
  }
  for (const cell of requiredCells) {
    const row = reported.get(cell.id)
    if (!row) { errors.push(`missing qualification cell ${cell.id}`); continue }
    if (row.outcome !== "Passed") {
      errors.push(`${cell.id}: ${row.outcome ?? "missing outcome"} (required qualification cell)`)
      continue
    }
    if (!Array.isArray(row.evidence) || !row.evidence.length) {
      errors.push(`${cell.id}: no qualification evidence`)
      continue
    }
    for (const evidence of row.evidence) {
      validateEvidenceRow(`${cell.id}: `, evidence, manifest, candidateDigest, verifyFile, errors)
    }
  }
  return errors
}

function main() {
  const args = process.argv.slice(2)
  const expected = inventory()
  if (args.length === 1 && args[0] === "--inventory") {
    process.stdout.write(`${expected.audit.length} audit issues; ${expected.priorReview.length} prior-review; ${expected.discriminators.length} discriminators; ${expected.gates.length - 17} child families; 17 parent gates\n`)
    return
  }
  if (args.length === 1 && args[0] === "--candidate-digest") {
    process.stdout.write(`${computeCandidateSourceDigest()}\n`)
    return
  }
  if (args.length === 1 && args[0] === "--specification-revision") {
    process.stdout.write(`${expected.specificationRevision}\n`)
    return
  }
  if (args[0] === "--write-native-provenance" && args.length <= 2) {
    const stamp = writeNativeProvenance(args[1])
    process.stdout.write(`${stamp.artifact.path} ${stamp.artifact.sha256}\n`)
    return
  }
  if (args[0] === "--verify-native-provenance" && args.length <= 2) {
    const stamp = verifyNativeProvenance(args[1])
    process.stdout.write(`${stamp.artifact.path} matches candidate ${stamp.candidateSourceDigest.slice(0, 12)}…\n`)
    return
  }
  const [phase, file, digestConfirm] = args
  if (!["pre-promotion", "post-promotion"].includes(phase) || args.length > 3) {
    throw Error("Usage: node scripts/release-results.mjs --inventory | --candidate-digest | --specification-revision | --write-native-provenance [binary] | --verify-native-provenance [binary] | pre-promotion|post-promotion [manifest.json] [exact-candidate-source-digest]")
  }
  const candidateDigest = computeCandidateSourceDigest()
  if (digestConfirm && digestConfirm !== candidateDigest) {
    throw Error("candidate source digest confirmation does not match recomputed inventory")
  }
  const manifestPath = file ?? RESULTS_PATH
  if (!fs.existsSync(path.resolve(root, manifestPath))) {
    process.stderr.write(`Release NOT qualified: no candidate evidence at ${manifestPath}.\n`)
    process.exitCode = 1
    return
  }
  const manifest = JSON.parse(read(manifestPath))
  const errors = validateResults(manifest, expected, phase, fileRef => {
    try { return crypto.createHash("sha256").update(fs.readFileSync(path.resolve(root, fileRef.path))).digest("hex") === fileRef.sha256 } catch { return false }
  }, { candidateSourceDigest: candidateDigest })
  if (errors.length) {
    process.stderr.write(`Release NOT qualified: ${errors.length} unresolved checks.\n${errors.slice(0, 12).join("\n")}\n`)
    process.exitCode = 1
  } else process.stdout.write(`Release evidence complete for candidate ${candidateDigest.slice(0, 12)}… (${phase}); no publication performed.\n`)
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
