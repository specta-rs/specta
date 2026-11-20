// Asks the model for a patch. It never runs cargo: the reproduction and the
// test suite execute in a separate job with no credentials.

import path from "node:path";

import { askJson, prepare } from "./lib/opencode.mjs";
import { b64, optional, required, setOutput, summary, unb64 } from "./lib/env.mjs";
import { inspectPatch } from "./lib/patch.mjs";
import { validateSpec } from "./lib/repro-spec.mjs";
import { escapeModelText, wrapUntrusted } from "./lib/untrusted.mjs";

const SCHEMA = {
  type: "object",
  fields: {
    diagnosis: { type: "string", max: 2_000 },
    patch: { type: "string", max: 96 * 1024 },
    notes: { type: "string", max: 1_500 },
    injection_suspected: { type: "boolean" },
  },
};

async function main() {
  const root = path.resolve(process.argv[2] ?? "repo");
  const issueNumber = Number(required("ISSUE_NUMBER"));
  const spec = validateSpec(JSON.parse(unb64(required("REPRO_SPEC_B64"))));
  const runLog = unb64(optional("REPRO_LOG_B64"));
  const issueBody = unb64(optional("ISSUE_BODY_B64"));

  prepare(root, path.join(root, ".github/issue-triage"));

  const user = [
    `<repository>${optional("REPO", "specta-rs/specta")}</repository>`,
    `<issue_number>${issueNumber}</issue_number>`,
    "<report>",
    wrapUntrusted("issue", issueBody, { max: 8_000 }),
    "</report>",
    "",
    "<reproduction>",
    wrapUntrusted(
      "reproduction",
      [
        `kind: ${spec.kind}`,
        `expected failure: ${spec.why}`,
        "",
        ...spec.files.map((file) => `--- ${file.path}\n${file.content}`),
      ].join("\n"),
      { max: 20_000 }
    ),
    "</reproduction>",
    "",
    "<sandbox_run_log>",
    wrapUntrusted("tool_output", runLog, { max: 10_000 }),
    "</sandbox_run_log>",
    "",
    "Produce the patch that fixes this. Answer with the JSON object described in the schema.",
  ].join("\n");

  const result = await askJson({
    root,
    agent: "fix",
    prompt: user,
    schema: SCHEMA,
    session: `ses_fix_${issueNumber}`,
    extraChecks: (parsed) => {
      if (String(parsed.patch ?? "").trim() === "") return [];
      try {
        inspectPatch(parsed.patch);
        return [];
      } catch (error) {
        return [`$.patch: ${error.message}`];
      }
    },
  });

  let ok = String(result.patch ?? "").trim() !== "";
  let reason = ok ? "" : "the model did not produce a patch";
  if (ok) {
    try {
      inspectPatch(result.patch);
    } catch (error) {
      ok = false;
      reason = error.message;
    }
  }

  setOutput("ok", ok ? "true" : "false");
  setOutput("patch", ok ? b64(result.patch) : "");
  setOutput("diagnosis", b64(result.diagnosis ?? ""));
  setOutput("notes", b64(result.notes ?? ""));
  if (!ok) setOutput("reason", reason);

  summary(
    `\n### Fix attempt\n\n${ok ? "Patch prepared." : `No usable patch: ${escapeModelText(reason, { max: 300 })}`}\n\n` +
      `> ${escapeModelText(result.diagnosis, { max: 600 })}\n`
  );
  console.log(ok ? "patch prepared" : `patch rejected: ${reason}`);
}

main().catch((error) => {
  console.error(`::error::${error.message}`);
  process.exit(1);
});
