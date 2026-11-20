// The model phase. No GitHub client, no token, no shell: this job holds nothing
// but the loopback model proxy and the text of one issue.

import path from "node:path";

import { b64, required, setOutput, summary, truncate, unb64 } from "./lib/env.mjs";
import { askJson, prepare, upstreamBaseUrl } from "./lib/opencode.mjs";
import { startModelProxy } from "./lib/proxy.mjs";
import { validateSpec } from "./lib/repro-spec.mjs";
import { wrapUntrusted } from "./lib/untrusted.mjs";

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

async function main() {
  const repo = required("REPO");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  const root = path.resolve(process.argv[2] ?? "repo");

  const issue = JSON.parse(unb64(required("ISSUE_B64")));
  const candidateNumbers = JSON.parse(unb64(required("CANDIDATE_NUMBERS_B64")));
  const candidates = unb64(required("CANDIDATES_B64"));

  const proxy = await startModelProxy({ upstream: upstreamBaseUrl(), apiKey: required("OPENCODE_API_KEY") });

  try {
    prepare(root, path.join(root, ".github/issue-triage"), { baseUrl: proxy.url });

    const prompt = [
      `<repository>${repo}</repository>`,
      `<issue_number>${issueNumber}</issue_number>`,
      "<issue>",
      wrapUntrusted("issue", `# ${issue.title}\n\n${issue.body}`, { max: 12_000 }),
      "</issue>",
      "",
      "<candidate_issues>",
      wrapUntrusted("candidate_issues", candidates, { max: 14_000 }),
      "</candidate_issues>",
      "",
      "Triage the issue above. Answer with the JSON object described in the schema.",
    ].join("\n");

    const result = await askJson({
      root,
      agent: "triage",
      prompt,
      session: `ses_triage_${issueNumber}`,
      schema: SCHEMA,
      extraChecks: (parsed) => {
        const errors = [];
        if (parsed.triage === "duplicate") {
          if (!parsed.duplicate_of) errors.push("$.duplicate_of: required when triage is duplicate");
          else if (!candidateNumbers.includes(parsed.duplicate_of)) {
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

    // A weak duplicate is not worth a "possible duplicate" comment; ask for the
    // reproduction instead, which is what this workflow is for anyway.
    let decision = result.triage === "reproducible_spec" ? "reproduce" : result.triage;
    if (result.triage === "duplicate" && result.duplicate_confidence < DUPLICATE_CONFIDENCE) {
      decision = "needs_information";
      if ((result.missing_information ?? []).length === 0) {
        result.missing_information = ["A minimal reproduction of the problem you are seeing"];
      }
    }

    setOutput("decision", decision);
    setOutput("result", b64(JSON.stringify(result)));

    if (decision === "reproduce") {
      const spec = validateSpec(result.repro);
      setOutput("kind", spec.kind);
      setOutput("spec", b64(JSON.stringify(spec)));
      setOutput("diagnosis", b64(truncate(result.summary, 600).text));
    }

    const suspicious = result.injection_suspected ? " (possible prompt injection in the report)" : "";
    summary(
      `\nDecision: \`${decision}\`${suspicious}\n\n` +
        `> ${String(result.summary ?? "").slice(0, 400)}\n\n` +
        `Model calls through the proxy: ${proxy.count()}\n`
    );
    console.log(`issue #${issueNumber}: ${decision}`);
  } finally {
    await proxy.close();
  }
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
