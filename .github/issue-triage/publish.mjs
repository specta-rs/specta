// Applies an already-verified patch, pushes a branch and opens a draft pull
// request. This job never runs code from the report or the patch; `git apply`
// only writes files, and the path allowlist is re-checked here rather than
// trusted from an earlier job.

import { execFileSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import { fixedComment } from "./lib/comment.mjs";
import { required, optional, setOutput, summary, unb64 } from "./lib/env.mjs";
import { branchExists, commentOnIssue, createDraftPullRequest, getIssue, gh, pullUrl } from "./lib/gh.mjs";
import { inspectPatch } from "./lib/patch.mjs";
import { escapeModelText } from "./lib/untrusted.mjs";

function git(cwd, args, options = {}) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
    env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
    ...options,
  });
}

function subject(diagnosis, issueNumber) {
  const first = String(diagnosis ?? "").split(/[.\n]/)[0] ?? "";
  const cleaned = first
    .replace(/[^\p{L}\p{N} _\-/#:()'"]/gu, "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 60)
    .replace(/["\\]/g, "");
  return cleaned || `reported behaviour in issue #${issueNumber}`;
}

async function main() {
  const repo = required("REPO");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw new Error("GITHUB_TOKEN is not available");

  const patch = unb64(required("PATCH_B64"));
  const diagnosis = unb64(optional("DIAGNOSIS_B64"));
  const notes = unb64(optional("NOTES_B64"));
  const { files, added } = inspectPatch(patch);

  const repository = await gh(`/repos/${repo}`, { token });
  const base = repository.default_branch || "main";

  const workdir = mkdtempSync(path.join(process.env.RUNNER_TEMP || os.tmpdir(), "specta-publish-"));
  const auth = Buffer.from(`x-access-token:${token}`).toString("base64");
  const gitAuth = ["-c", `http.https://github.com/.extraheader=AUTHORIZATION: basic ${auth}`];

  git(workdir, [...gitAuth, "clone", "--quiet", "--depth", "1", `https://github.com/${repo}.git`, "repo"]);
  const clone = path.join(workdir, "repo");

  git(clone, ["config", "user.name", "github-actions[bot]"]);
  git(clone, ["config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com"]);

  // A re-run should not collide with the branch a previous run pushed.
  let branch = `triage/issue-${issueNumber}`;
  for (let attempt = 2; attempt <= 5 && (await branchExists(repo, branch)); attempt++) {
    branch = `triage/issue-${issueNumber}-${attempt}`;
  }
  git(clone, ["checkout", "-b", branch]);

  execFileSync("git", ["apply", "--whitespace=nowarn", "--index", "-"], {
    cwd: clone,
    input: patch,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });

  const title = `fix: ${subject(diagnosis, issueNumber)} (#${issueNumber})`;
  git(clone, ["commit", "--quiet", "-m", `${title}\n\nAutomated fix for issue #${issueNumber}.`]);
  git(clone, [...gitAuth, "push", "--quiet", "origin", `HEAD:refs/heads/${branch}`]);

  const body = [
    `Draft fix for #${issueNumber}, produced by \`.github/workflows/issue-triage.yml\`.`,
    "",
    "> **This patch was written by a model and has not been reviewed.** The workflow only verified that the reproduction now passes and that the workspace test suite is green.",
    "",
    `Reproduction kind: \`${optional("REPRO_KIND", "runtime")}\`. Files: ${files.length}. Added lines: ${added}.`,
    "",
    "### Diagnosis",
    "",
    escapeModelText(diagnosis, { max: 1_800 }),
    notes ? `\n### Notes\n\n${escapeModelText(notes, { max: 1_200 })}` : "",
    "",
    "### Before merging",
    "",
    "- [ ] Check the diagnosis is about the reported defect and not a nearby one.",
    "- [ ] Confirm the regression test actually fails without the source change.",
    "- [ ] Run CI. Pull requests opened with the repository token do not trigger workflows, so CI has not run on this branch.",
    "",
    "<details><summary>Why the workflow cannot open a ready pull request</summary>",
    "",
    "The workflow runs with no secrets and no OAuth app, so it uses the repository token. That token cannot trigger further workflow runs, and every patch is therefore a draft for a human to promote.",
    "",
    "</details>",
  ]
    .filter((line) => line !== "")
    .join("\n");

  const pull = await createDraftPullRequest(repo, { title, head: branch, base, body });
  setOutput("pull", String(pull.number));
  summary(`\nOpened draft pull request #${pull.number} for #${issueNumber}.\n`);

  const issue = await getIssue(repo, issueNumber);
  const comments = [];
  if (!issue.pull_request) {
    try {
      await commentOnIssue(
        repo,
        issueNumber,
        fixedComment({ repo, number: issueNumber, pull: pull.number, summary: diagnosis })
      );
    } catch (error) {
      comments.push(error.message);
    }
  }

  console.log(`opened ${pullUrl(repo, pull.number)}`);
  if (comments.length) console.error(`::warning::comment failed: ${comments.join("; ")}`);
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
