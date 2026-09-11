import { execFileSync } from "node:child_process"
import { createHash } from "node:crypto"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { listCandidateSourcePaths } from "./release-results.mjs"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const marker = ".git/bumbledb-build.json"
const platforms = ["darwin-arm64", "linux-arm64", "linux-x64"]
const sha256 = file => createHash("sha256").update(fs.readFileSync(file)).digest("hex")

function git(cwd, args, options = {}) {
  return execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", ...options }).trim()
}

function command(cwd, executable, args, env = {}) {
  execFileSync(executable, args, {
    cwd, stdio: "inherit",
    env: { ...process.env, ...env, NODE_PATH: "", NODE_OPTIONS: "" },
  })
}

/** Select Git objects once. The caller's index, refs and working files stay untouched. */
export function snapshotSource(sourceRoot, destination) {
  const selection = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-selection-"))
  try {
    const env = { ...process.env, GIT_INDEX_FILE: path.join(selection, "index") }
    const revision = git(sourceRoot, ["rev-parse", "HEAD"])
    const paths = listCandidateSourcePaths(sourceRoot)
    fs.writeFileSync(path.join(selection, "paths"), `${paths.join("\0")}\0`)
    git(sourceRoot, ["read-tree", revision], { env })
    git(sourceRoot, ["add", "-A", `--pathspec-from-file=${path.join(selection, "paths")}`, "--pathspec-file-nul"], { env })
    const tree = git(sourceRoot, ["write-tree"], { env })
    const commit = git(sourceRoot, [
      "-c", "user.name=BumbleDB build", "-c", "user.email=build@localhost",
      "commit-tree", tree,
    ], { input: "Immutable build input\n" })
    fs.mkdirSync(destination, { recursive: true })
    git(destination, ["init", "-q"])
    git(destination, ["fetch", "-q", "--depth=1", sourceRoot, commit])
    git(destination, ["checkout", "-q", "--detach", "FETCH_HEAD"])
    for (const rel of listCandidateSourcePaths(destination)) {
      const file = path.join(destination, rel)
      if (fs.lstatSync(file).isSymbolicLink()) {
        const resolved = fs.realpathSync(file)
        if (!resolved.startsWith(`${fs.realpathSync(destination)}${path.sep}`)) {
          throw Error(`build input symlink escapes the selected source: ${rel}`)
        }
      }
    }
    const source = { tree, revision }
    fs.writeFileSync(path.join(destination, marker), `${JSON.stringify(source)}\n`)
    return source
  } finally {
    fs.rmSync(selection, { recursive: true, force: true })
  }
}

/** Only the selected source may supply build inputs or stamp its artifacts. */
export function checkBuildSource(sourceRoot = root) {
  const source = JSON.parse(fs.readFileSync(path.join(sourceRoot, marker), "utf8"))
  if (git(sourceRoot, ["rev-parse", "HEAD^{tree}"]) !== source.tree) throw Error("build source tree changed")
  git(sourceRoot, ["diff", "--exit-code", "HEAD", "--"])
  const untracked = git(sourceRoot, ["ls-files", "--others", "--exclude-standard"])
  if (untracked) throw Error(`unselected build inputs: ${untracked}`)
  return source
}

/** Verify a completed family against the exact release checkout and its bytes. */
export function checkFamily(out, sourceRoot = root) {
  const family = JSON.parse(fs.readFileSync(path.join(out, "family.json"), "utf8"))
  const selected = checkBuildSource(path.join(out, "source"))
  if (family.tree !== selected.tree || family.revision !== selected.revision ||
      family.tree !== git(sourceRoot, ["rev-parse", "HEAD^{tree}"]) ||
      family.revision !== git(sourceRoot, ["rev-parse", "HEAD"])) {
    throw Error("package family does not match the release commit")
  }
  const packages = path.join(out, "packages")
  const files = fs.readdirSync(packages).filter(file => file.endsWith(".tgz")).sort()
  if (files.length !== 5 || JSON.stringify(files) !== JSON.stringify(Object.keys(family.hashes).sort())) {
    throw Error("release requires exactly five recorded packages")
  }
  for (const file of files) {
    if (sha256(path.join(packages, file)) !== family.hashes[file]) throw Error(`package changed: ${file}`)
  }
  return family
}

function runBuild(sourceRoot, nativeArtifacts, allPlatforms) {
  const env = { CARGO_TARGET_DIR: path.join(sourceRoot, "ts/crate/target") }
  for (const name of ["ts", "ts-log"]) {
    command(path.join(sourceRoot, name), "pnpm", ["install", "--frozen-lockfile"], env)
  }
  command(path.join(sourceRoot, "ts"), "pnpm", ["exec", "node", "scripts/build.ts"], env)
  command(path.join(sourceRoot, "ts-log"), "pnpm", ["exec", "node", "scripts/build.ts"], env)
  if (nativeArtifacts) {
    for (const platform of allPlatforms ? platforms : [`${process.platform}-${process.arch}`]) {
      const target = path.join(sourceRoot, "ts/npm", platform)
      // This is private staging; no loaded/live addon is overwritten.
      fs.copyFileSync(path.join(nativeArtifacts, `bumbledb.${platform}.node`), path.join(target, "bumbledb.node"))
      fs.copyFileSync(path.join(nativeArtifacts, `bumbledb.${platform}.provenance.json`), path.join(target, ".native-provenance.json"))
    }
  }
  checkBuildSource(sourceRoot)
  command(sourceRoot, "sh", ["scripts/packed-import.sh", ...(allPlatforms ? [] : ["--host-only"])], env)
}

/** The only publication of local artifacts is one completed family directory. */
export function buildFamily({ sourceRoot = root, out, nativeArtifacts, allPlatforms = false, build = runBuild }) {
  out = path.resolve(out)
  if (out === sourceRoot || out.startsWith(`${sourceRoot}${path.sep}`)) throw Error("build output must be outside the source checkout")
  if (fs.existsSync(out)) throw Error(`build output already exists: ${out}`)
  fs.mkdirSync(path.dirname(out), { recursive: true })
  const attempt = fs.mkdtempSync(path.join(path.dirname(out), ".bumbledb-build-"))
  try {
    const selectedRoot = path.join(attempt, "source")
    const selected = snapshotSource(sourceRoot, selectedRoot)
    build(selectedRoot, nativeArtifacts, allPlatforms)
    checkBuildSource(selectedRoot)
    const packages = path.join(attempt, "packages")
    fs.mkdirSync(packages)
    command(selectedRoot, "node", ["ts/scripts/stage.ts", "--out", packages, ...(allPlatforms ? [] : ["--host-only"])])
    command(selectedRoot, "node", ["ts-log/scripts/stage.ts", "--out", packages])
    const files = fs.readdirSync(packages).filter(file => file.endsWith(".tgz")).sort()
    if (files.length !== (allPlatforms ? 5 : 3)) throw Error("incomplete package family")
    const hashes = Object.fromEntries(files.map(file => [file, sha256(path.join(packages, file))]))
    fs.writeFileSync(path.join(packages, "SHA256SUMS"), files.map(file => `${hashes[file]}  ${file}\n`).join(""))
    fs.writeFileSync(path.join(attempt, "family.json"), `${JSON.stringify({ ...selected, hashes }, null, 2)}\n`)
    checkBuildSource(selectedRoot)
    fs.renameSync(attempt, out)
    return out
  } catch (error) {
    fs.rmSync(attempt, { recursive: true, force: true })
    throw error
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(fs.realpathSync(process.argv[1])).href) {
  const args = process.argv.slice(2)
  if (args.length === 1 && args[0] === "--check-source") {
    checkBuildSource()
  } else if (args.length === 2 && args[0] === "--check-family") {
    checkFamily(path.resolve(args[1]))
  } else {
    const options = { sourceRoot: root }
    for (let index = 0; index < args.length; index++) {
      switch (args[index]) {
        case "--out": options.out = args[++index]; break
        case "--native-artifacts": options.nativeArtifacts = path.resolve(args[++index]); break
        case "--all-platforms": options.allPlatforms = true; break
        default: throw Error(`unknown build argument: ${args[index]}`)
      }
    }
    options.out ??= path.join(fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-family-")), "ready")
    console.log(`Completed package family: ${buildFamily(options)}`)
  }
}
