#!/usr/bin/env node
// Release builds and their full correctness batteries must finish successfully.
// The workflow's aggregate conclusion also includes optional static/Miri jobs.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { pathToFileURL, fileURLToPath } from "node:url";

export const requiredJobs = ["check / darwin", "check / linux-arm64", "check / linux-x64"];

export function releaseProblems(run, revision) {
  const problems = [];
  if (run.headSha !== revision) problems.push("run does not match the candidate commit");
  if (run.workflowName !== "ci" || run.event !== "push" || run.headBranch !== "main") {
    problems.push("expected the ci push run on main");
  }
  for (const name of requiredJobs) {
    const matches = (run.jobs ?? []).filter((job) => job.name === name);
    if (matches.length !== 1) {
      problems.push(`${name}: missing or ambiguous`);
    } else if (matches[0].status !== "completed" || matches[0].conclusion !== "success") {
      problems.push(`${name}: ${matches[0].status} / ${matches[0].conclusion || "pending"}`);
    }
  }
  return problems;
}

function main() {
  const [runId] = process.argv.slice(2);
  if (!/^\d+$/.test(runId ?? "") || process.argv.length !== 3) {
    throw new Error("usage: node scripts/release-ready.mjs <ci-run-id>");
  }
  const cwd = fileURLToPath(new URL("..", import.meta.url));
  const command = (exe, args) => execFileSync(exe, args, { cwd, encoding: "utf8" }).trim();
  if (command("git", ["status", "--porcelain"])) throw new Error("candidate checkout must be clean");
  const revision = command("git", ["rev-parse", "HEAD"]);
  const run = JSON.parse(command("gh", ["run", "view", runId, "--json",
    "headSha,headBranch,event,workflowName,jobs,url,status,conclusion"]));
  const problems = releaseProblems(run, revision);
  if (problems.length) throw new Error(problems.join("\n"));
  console.log(`Release CI ready: ${revision}\n${run.url}\n${requiredJobs.join("\n")}\nStatic Linux and Miri are not release blockers.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(fs.realpathSync(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
