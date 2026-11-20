// The only job that reads the issue, and the only reason it holds a token.
//
// It does not run a model and it does not execute anything. Everything it reads
// is handed to the agent job through step outputs, so the agent never needs
// GitHub access at all.

import { b64, required, setOutput, summary, truncate } from "./lib/env.mjs";
import { getIssue, issueUrl, listComments, listRecentIssues } from "./lib/gh.mjs";
import { MARKER } from "./lib/comment.mjs";
import { candidateDigest, rankCandidates } from "./lib/similar.mjs";

const MAX_ISSUE_CHARS = 12_000;

function finish(decision, reason) {
  setOutput("decision", decision);
  if (reason) summary(`\nDecision: \`${decision}\` — ${reason}\n`);
  console.log(`${decision}: ${reason}`);
}

async function main() {
  const repo = required("REPO");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  if (!Number.isInteger(issueNumber) || issueNumber <= 0) {
    console.error("::error::ISSUE_NUMBER is not a positive integer");
    process.exit(1);
  }

  summary(`### Issue #${issueNumber}\n\n`);

  const issue = await getIssue(repo, issueNumber);
  if (issue.pull_request) return finish("skip", "this is a pull request");
  if ((issue.labels ?? []).some((label) => ["triage:skip", "triage:ignore"].includes(label.name))) {
    return finish("skip", "the triage:skip label is set");
  }
  if (String(issue.user?.login ?? "").endsWith("[bot]") || issue.user?.type === "Bot") {
    return finish("skip", "the issue was opened by a bot");
  }

  const comments = await listComments(repo, issueNumber);
  const forced = (process.env.TRIAGE_FORCE ?? "false") === "true";
  if (!forced && comments.some((comment) => String(comment.body ?? "").includes(MARKER))) {
    return finish("skip", "this issue already has a triage comment");
  }

  const title = String(issue.title ?? "");
  const body = String(issue.body ?? "");
  if (body.trim().length < 40) return finish("needs_info", "the report is too short to build a reproduction from");

  const candidates = rankCandidates({ number: issueNumber, title, body }, await listRecentIssues(repo));

  setOutput("issue", b64(JSON.stringify({ title, body: truncate(body, MAX_ISSUE_CHARS).text })));
  setOutput("candidates", b64(candidateDigest(candidates)));
  setOutput("candidate_numbers", b64(JSON.stringify(candidates.map((candidate) => candidate.number))));
  finish("triage", `${candidates.length} candidate issue(s) for the duplicate check`);
  console.log(issueUrl(repo, issueNumber));
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
