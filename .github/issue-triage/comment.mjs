// Posts the outcome that is not a pull request.
//
// This job holds `issues: write` and nothing else: no model, no secret, and no
// code from the report. Every string it prints comes from a template in
// `lib/comment.mjs`, so the model cannot mention anyone or link anywhere.

import {
  MARKER,
  cannotReproduceComment,
  duplicateComment,
  fixFailedComment,
  needsInfoComment,
  notABugComment,
  verifyFailedComment,
} from "./lib/comment.mjs";
import { optional, required, summary, unb64 } from "./lib/env.mjs";
import { commentOnIssue, getIssue, listComments } from "./lib/gh.mjs";

// Decides what, if anything, this issue should be told. `null` means the
// workflow has either already handled it or is about to open a pull request.
function pickScenario() {
  const collect = optional("COLLECT_DECISION");
  const triage = optional("TRIAGE_DECISION");

  if (collect === "needs_info") return "needs-information";
  if (collect !== "triage") return null;

  if (triage === "duplicate") return "duplicate";
  if (triage === "needs_information") return "needs-information";
  if (triage === "not_a_bug") return "not-a-bug";

  if (triage === "reproduce") {
    if (optional("REPRODUCED") !== "true") return "cannot-reproduce";
    if (optional("FIX_OK") !== "true") return "fix-failed";
    if (optional("VERIFIED") !== "true") return "verify-failed";
  }

  return null;
}

function parseResult() {
  const raw = optional("RESULT_B64");
  if (!raw) return {};
  try {
    return JSON.parse(unb64(raw));
  } catch {
    return {};
  }
}

async function main() {
  const repo = required("REPO");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  const scenario = pickScenario();

  if (!scenario) {
    console.log("nothing to comment on");
    return;
  }
  summary(`\nCommenting: \`${scenario}\`\n`);

  const issue = await getIssue(repo, issueNumber);
  if (issue.pull_request) return;

  const forced = optional("TRIAGE_FORCE") === "true";
  const comments = await listComments(repo, issueNumber);
  if (!forced && comments.some((comment) => String(comment.body ?? "").includes(MARKER))) {
    console.log("issue already has a triage comment, nothing to do");
    return;
  }

  const result = parseResult();
  const runUrl = optional("RUN_URL");
  const base = { repo, number: issueNumber, summary: result.summary };

  let body;
  switch (scenario) {
    case "duplicate":
      body = duplicateComment({
        ...base,
        duplicateOf: result.duplicate_of,
        confidence: result.duplicate_confidence,
      });
      break;
    case "needs-information":
      body = needsInfoComment({
        ...base,
        summary: result.summary ?? "The report does not contain enough detail to build a reproduction from.",
        missing:
          result.missing_information?.length > 0
            ? result.missing_information
            : [
                "The code that triggers the problem",
                "The version or commit you are on",
                "What you expected and what happened instead",
              ],
      });
      break;
    case "not-a-bug":
      body = notABugComment(base);
      break;
    case "cannot-reproduce":
      body = cannotReproduceComment({
        ...base,
        kind: optional("REPRO_KIND", "runtime"),
        log: unb64(optional("REPRO_LOG_B64")),
      });
      break;
    case "fix-failed":
      body = fixFailedComment({ ...base, runUrl });
      break;
    case "verify-failed":
      body = verifyFailedComment({ ...base, runUrl });
      break;
    default:
      throw new Error(`unknown scenario ${scenario}`);
  }

  await commentOnIssue(repo, issueNumber, body);
  console.log(`commented on #${issueNumber}: ${scenario}`);
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
