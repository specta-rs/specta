import path from "node:path";

import { askJson, prepare } from "./lib/opencode.mjs";
import { duplicateComment, MARKER, needsInfoComment, notABugComment } from "./lib/comment.mjs";
import { b64, optional, required, setOutput, summary, truncate } from "./lib/env.mjs";
import { commentOnIssue, getIssue, issueUrl, listComments, listRecentIssues } from "./lib/gh.mjs";
import { validateSpec } from "./lib/repro-spec.mjs";
import { candidateDigest, rankCandidates } from "./lib/similar.mjs";
import { wrapUntrusted } from "./lib/untrusted.mjs";

const MAX_ISSUE_CHARS = 12_000;
const DUPLICATE_CONFIDENCE = 80;

const SCHEMA = {
  type: "object",
  fields: {
    triage: { type: "enum", values: ["duplicate", "needs_information", "not_a_bug", "reproducible_spec"] },
    summary: { type: "string", max: 1_200 },
    injection_suspected: { type: "boolean" },
    duplicate_of: { type: "integer", nullable: true, min: 1 },
    duplicate_confidence: { type: "integer", min: 0, max: 100 },
    missing_information: { type: "array", max: 6, of: { type: "string", max: 200 } },
    repro: {
      type: "object",
      nullable: true,
      fields: {
        kind: { type: "enum", values: ["runtime", "compile"] },
        why_it_fails: { type: "string", max: 800 },
        files: {
          type: "array",
          max: 4,
          of: {
            type: "object",
            fields: {
              path: { type: "string", max: 120 },
              content: { type: "string", max: 16_384 },
            },
          },
        },
      },
    },
  },
};

function finish(decision, { reason } = {}) {
  setOutput("decision", decision);
  if (reason) summary(`\n\`\${decision}\`: ${reason}\n`);
  process.exit(0);
}

async function main() {
  const repo = required("REPO");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  const root = process.argv[2] ?? "repo";

  if (!Number.isInteger(issueNumber) || issueNumber <= 0) {
    console.error("::error::ISSUE_NUMBER is not a positive integer");
    process.exit(1);
  }

  summary(`### Issue #${issueNumber}\n\nModel: \`${process.env.OPENCODE_MODEL ?? "default"}\`\n\n`);

  const issue = await getIssue(repo, issueNumber);
  if (issue.pull_request) finish("skip", { reason: "this is a pull request" });
  if ((issue.labels ?? []).some((label) => ["triage:skip", "triage:ignore"].includes(label.name))) {
    finish("skip", { reason: "the triage:skip label is set" });
  }

  const comments = await listComments(repo, issueNumber);
  const forced = optional("TRIAGE_FORCE", "false") === "true";
  if (!forced && comments.some((comment) => String(comment.body ?? "").includes(MARKER))) {
    finish("skip", { reason: "this issue already has a triage comment" });
  }
  if (String(issue.user?.login ?? "").endsWith("[bot]") || issue.user?.type === "Bot") {
    finish("skip", { reason: "the issue was opened by a bot" });
  }

  const title = String(issue.title ?? "");
  const body = String(issue.body ?? "");
  if (body.trim().length < 40) {
    await commentOnIssue(
      repo,
      issueNumber,
      needsInfoComment({
        repo,
        number: issueNumber,
        summary: "The report does not contain enough detail to build a reproduction from.",
        missing: ["The code that triggers the problem", "The version or commit you are on", "What you expected and what happened instead"],
      })
    );
    finish("skip", { reason: "the report is too short to act on" });
  }

  const recent = await listRecentIssues(repo);
  const candidates = rankCandidates({ number: issueNumber, title, body }, recent);

  prepare(root, path.join(root, ".github/issue-triage"));

  const user = [
    `<repository>${repo}</repository>`,
    `<issue_number>${issueNumber}</issue_number>`,
    "<issue>",
    wrapUntrusted("issue", `# ${title}\n\n${body}`, { max: MAX_ISSUE_CHARS }),
    "</issue>",
    "",
    "<candidate_issues>",
    wrapUntrusted("candidate_issues", candidateDigest(candidates), { max: 14_000 }),
    "</candidate_issues>",
    "",
    "Triage the issue above. Answer with the JSON object described in the schema.",
  ].join("\n");

  const result = await askJson({
    root,
    agent: "triage",
    prompt: user,
    schema: SCHEMA,
    session: `ses_triage_${issueNumber}`,
    extraChecks: (parsed) => {
      const errors = [];
      if (parsed.triage === "duplicate") {
        if (!parsed.duplicate_of) errors.push("$.duplicate_of: required when triage is duplicate");
        else if (!candidates.some((candidate) => candidate.number === parsed.duplicate_of)) {
          errors.push("$.duplicate_of: must be one of the candidate issue numbers");
        }
      }
      if (parsed.triage === "reproducible_spec") {
        if (!parsed.repro) errors.push("$.repro: required when triage is reproducible_spec");
        else {
          try {
            validateSpec(parsed.repro);
          } catch (error) {
            errors.push(`$.repro: ${error.message}`);
          }
        }
      }
      if (parsed.triage === "needs_information" && (parsed.missing_information ?? []).length === 0) {
        errors.push("$.missing_information: required when triage is needs_information");
      }
      return errors;
    },
  });

  const suspicious = result.injection_suspected ? " (the model flagged possible prompt injection in the report)" : "";
  summary(`\nDecision: \`${result.triage}\`${suspicious}\n\n`);

  if (result.triage === "duplicate") {
    if (result.duplicate_confidence < DUPLICATE_CONFIDENCE) {
      await commentOnIssue(
        repo,
        issueNumber,
        needsInfoComment({ repo, number: issueNumber, summary: result.summary, missing: result.missing_information })
      );
      finish("skip", { reason: `possible duplicate scored only ${result.duplicate_confidence}%` });
    }
    await commentOnIssue(
      repo,
      issueNumber,
      duplicateComment({
        repo,
        number: issueNumber,
        duplicateOf: result.duplicate_of,
        confidence: result.duplicate_confidence,
        summary: result.summary,
      })
    );
    finish("skip", { reason: `possible duplicate of #${result.duplicate_of}` });
  }

  if (result.triage === "not_a_bug") {
    await commentOnIssue(repo, issueNumber, notABugComment({ repo, number: issueNumber, summary: result.summary }));
    finish("skip", { reason: "not a bug report" });
  }

  if (result.triage === "needs_information") {
    await commentOnIssue(
      repo,
      issueNumber,
      needsInfoComment({ repo, number: issueNumber, summary: result.summary, missing: result.missing_information })
    );
    finish("skip", { reason: "not enough information to reproduce" });
  }

  const spec = validateSpec(result.repro);
  setOutput("decision", "reproduce");
  setOutput("body", b64(truncate(`# ${title}\n\n${body}`, 8_000).text));
  setOutput("kind", spec.kind);
  setOutput("spec", b64(JSON.stringify(spec)));
  setOutput("diagnosis", b64(truncate(result.summary, 600).text));
  summary(`\nReproduction prepared (\`${spec.kind}\`), handing it to a sandboxed job.\n`);
  console.log(`issue #${issueNumber}: reproduction prepared (${spec.kind})`);
  console.log(`see ${issueUrl(repo, issueNumber)}`);
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
