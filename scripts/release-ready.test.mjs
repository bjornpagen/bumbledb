import assert from "node:assert/strict";
import { test } from "node:test";
import { releaseProblems, requiredJobs } from "./release-ready.mjs";

function candidate() {
  return { headSha: "candidate", workflowName: "ci", event: "push", headBranch: "main",
    status: "in_progress", conclusion: null,
    jobs: requiredJobs.map((name) => ({ name, status: "completed", conclusion: "success" })) };
}

test("all platform batteries suffice while optional jobs run or fail", () => {
  for (const conclusion of [null, "failure", "cancelled", "skipped"]) {
    const run = candidate();
    run.conclusion = conclusion;
    run.jobs.push({ name: "static-linux-arm64 / musl", status: "in_progress", conclusion });
    run.jobs.push({ name: "miri", status: "completed", conclusion });
    assert.deepEqual(releaseProblems(run, "candidate"), []);
  }
});

test("every supported platform must finish successfully", () => {
  for (let index = 0; index < requiredJobs.length; index++) {
    for (const conclusion of [null, "failure", "cancelled", "skipped", "neutral", "timed_out"]) {
      const run = candidate();
      run.jobs[index].conclusion = conclusion;
      assert.equal(releaseProblems(run, "candidate").length, 1);
    }
    const running = candidate();
    running.jobs[index].status = "in_progress";
    assert.equal(releaseProblems(running, "candidate").length, 1);
    const missing = candidate();
    missing.jobs.splice(index, 1);
    assert.equal(releaseProblems(missing, "candidate").length, 1);
  }
});

test("old commits, wrong workflows and Miri-only dispatches cannot qualify", () => {
  for (const change of [{ headSha: "old" }, { workflowName: "other" },
    { event: "workflow_dispatch" }, { headBranch: "other" }, { jobs: [] }]) {
    assert.ok(releaseProblems({ ...candidate(), ...change }, "candidate").length);
  }
  const duplicate = candidate();
  duplicate.jobs.push(duplicate.jobs[0]);
  assert.equal(releaseProblems(duplicate, "candidate").length, 1);
});
