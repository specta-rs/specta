// Builds and runs the model-written reproduction, twice: once on the current
// source (`reproduce`) and once with the candidate fix applied (`verify`).
//
// This runs in a job with `permissions: {}` and no secret in the environment.
// `sandbox.sh` strips the remaining runner variables from the child process.
// Nothing here is allowed to touch the network, git, or the GitHub API.

import { spawnSync } from "node:child_process";
import path from "node:path";

import { b64, optional, required, setOutput, summary, truncate, unb64 } from "./lib/env.mjs";
import { validateSpec, writeSpec } from "./lib/repro-spec.mjs";

const mode = process.argv[2] ?? "reproduce";
const root = path.resolve(process.argv[3] ?? "repo");
const sandbox = path.join(root, ".github/issue-triage/sandbox.sh");
const TEST_TIMEOUT_SECONDS = 900;
const SUITE_TIMEOUT_SECONDS = 2_400;

function run(command, args, { timeoutSeconds = TEST_TIMEOUT_SECONDS, allowMissingTimeout = false } = {}) {
  const options = {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    env: process.env,
  };

  let result = spawnSync("timeout", ["--kill-after=20", String(timeoutSeconds), sandbox, command, ...args], options);
  if (allowMissingTimeout && result.error?.code === "ENOENT") {
    result = spawnSync(sandbox, [command, ...args], options);
  }
  if (result.error) throw result.error;

  return {
    status: result.status ?? 1,
    signal: result.signal ?? null,
    output: `${result.stdout ?? ""}${result.stderr ?? ""}`,
  };
}

function classify(kind, run) {
  if (run.status === 0) return "not-reproduced";

  const testFailed = /test result: FAILED|panicked at|thread .* panicked|assertion .* failed/im.test(run.output);
  const rustcError = /^error(\[E\d+\])?:/m.test(run.output);
  const linkError = /could not compile|error: could not compile/i.test(run.output);

  if (kind === "compile") {
    return rustcError || linkError ? "reproduced" : "inconclusive";
  }
  if (testFailed) return "reproduced";
  return "inconclusive";
}

function tail(output, lines = 120) {
  return String(output ?? "").split("\n").slice(-lines).join("\n");
}

function main() {
  const specB64 = required("REPRO_SPEC_B64");
  const spec = validateSpec(JSON.parse(unb64(specB64)));

  writeSpec(spec, root);
  console.log(`wrote reproduction (${spec.kind}) with ${spec.files.length} files`);

  if (mode === "verify") {
    const patch = unb64(required("PATCH_B64"));
    const applied = spawnSync("git", ["apply", "--whitespace=nowarn", "--index", "-"], {
      cwd: root,
      input: patch,
      encoding: "utf8",
      maxBuffer: 16 * 1024 * 1024,
      env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
    });
    if (applied.status !== 0) {
      const detail = `${applied.stdout ?? ""}${applied.stderr ?? ""}`.slice(0, 2_000);
      setOutput("verified", "false");
      setOutput("log", b64(`git apply failed\n${detail}`));
      summary(`\n### Verification failed\n\n\`git apply\` rejected the patch.\n`);
      console.log(detail);
      process.exit(0);
    }
  }

  const repro = run("cargo", ["test", "--manifest-path", "repro/Cargo.toml", "--", "--nocapture"], {
    allowMissingTimeout: true,
  });

  if (mode === "reproduce") {
    const classification = classify(spec.kind, repro);
    setOutput("reproduced", classification === "reproduced" ? "true" : "false");
    setOutput("kind", spec.kind);
    setOutput("log", b64(tail(repro.output, 200)));
    summary(
      `\n### Reproduction attempt\n\nKind: \`${spec.kind}\` · result: \`${classification}\` · exit: ${repro.status}\n\n` +
        `\`\`\`\n${truncate(tail(repro.output, 40), 3_000).text}\n\`\`\`\n`
    );
    console.log(`classification: ${classification} (exit ${repro.status})`);
    return;
  }

  const reproPassed = repro.status === 0;
  const fullSuite = optional("TRIAGE_FULL_SUITE", "true") !== "false";

  let suite = null;
  if (reproPassed && fullSuite) {
    console.log("reproduction passes, running the workspace test suite");
    suite = run("cargo", ["test", "--all-features"], {
      timeoutSeconds: SUITE_TIMEOUT_SECONDS,
      allowMissingTimeout: true,
    });
  }

  const verified = reproPassed && (suite === null || suite.status === 0);
  setOutput("verified", verified ? "true" : "false");
  setOutput("log", b64(tail(`${repro.output}\n${suite?.output ?? ""}`, 200)));
  summary(
    `\n### Verification\n\nReproduction: \`${reproPassed ? "passes" : "still fails"}\` ` +
      `(exit ${repro.status})\n\n` +
      (suite === null
        ? "Workspace suite: not run\n"
        : `Workspace suite: \`${suite.status === 0 ? "passes" : "fails"}\` (exit ${suite.status})\n`) +
      `\n\`\`\`\n${truncate(tail(`${repro.output}\n${suite?.output ?? ""}`, 40), 3_000).text}\n\`\`\`\n`
  );
  console.log(`verified: ${verified}`);
}

try {
  main();
} catch (error) {
  console.error(`::error::${error.message}`);
  process.exit(1);
}
