// Posts the outcome that is not a pull request. Runs with `issues: write` and
// no secret, and never executes code from the report.

import { MARKER, cannotReproduceComment, fixFailedComment, verifyFailedComment } from "./lib/comment.mjs";
import { required, summary, unb64, optional } from "./lib/env.mjs";
import { commentOnIssue, getIssue, listComments } from "./lib/gh.mjs";

async function main() {
  const repo = required("REPO");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  const scenario = process.argv[2];
  const log = unb64(optional("REPRO_LOG_B64"));
  const kind = optional("REPRO_KIND", "runtime");
  const runUrl = optional("RUN_URL");

  const issue = await getIssue(repo, issueNumber);
  if (issue.pull_request) return;

  const comments = await listComments(repo, issueNumber);
  if (comments.some((comment) => String(comment.body ?? "").includes(MARKER)) && optional("TRIAGE_FORCE") !== "true") {
    console.log("issue already has a triage comment, nothing to do");
    return;
  }

  if (scenario === "cannot-reproduce") {
    await commentOnIssue(
      repo,
      issueNumber,
      cannotReproduceComment({
        repo,
        number: issueNumber,
        kind,
        log,
        summary: "The reproduction built from this report did not fail on the current source.",
      })
    );
    summary(`\nCommented on #${issueNumber}: could not reproduce.\n`);
    return;
  }

  if (scenario === "fix-failed") {
    await commentOnIssue(
      repo,
      issueNumber,
      fixFailedComment({
        repo,
        number: issueNumber,
        runUrl,
        summary: "The bug reproduces, but the generated patch was rejected by the workflow's validation.",
      })
    );
    summary(`\nCommented on #${issueNumber}: reproduced without a usable fix.\n`);
    return;
  }

  if (scenario === "verify-failed") {
    await commentOnIssue(
      repo,
      issueNumber,
      verifyFailedComment({
        repo,
        number: issueNumber,
        runUrl,
        summary: "The generated patch did not make the reproduction pass without breaking the test suite.",
      })
    );
    summary(`\nCommented on #${issueNumber}: reproduced, patch did not verify.\n`);
    return;
  }

  console.error(`::error::unknown scenario ${scenario}`);
  process.exit(1);
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
