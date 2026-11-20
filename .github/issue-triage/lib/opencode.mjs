// The only place OpenCode is invoked.
//
// The agent runs from a clone whose configuration is written by `prepare()`,
// so the policy in `opencode.json` is the only one that exists at runtime. The
// agent has read, grep and glob; every other action — shell, edit, webfetch,
// websearch, skills, subagents, MCP tools — is denied by that file's wildcard
// rule, which also covers actions added in future OpenCode versions.
//
// Nothing here passes `--auto`, which would auto-approve any permission that is
// not explicitly denied.

import { spawn } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import { parseJsonLoose, validate } from "./schema.mjs";

const AGENTS = ["triage", "fix"];
const RUN_TIMEOUT_MS = 15 * 60 * 1000;

// The model process gets an allowlist, not the workflow's environment. That
// keeps `GITHUB_TOKEN`, `ACTIONS_RUNTIME_TOKEN` and any ambient OpenCode session
// out of the process that is being steered by a hostile issue.
//
// The real model key is deliberately absent: `startModelProxy` holds it and
// injects it in flight, so the agent sees a placeholder. Neither credential is
// present in the environment the agent runs under.
const PASS_THROUGH = [
  "PATH", "HOME", "TMPDIR", "TMP", "TEMP", "LANG", "LC_ALL", "LC_CTYPE", "TERM", "SHELL", "USER",
  "HTTPS_PROXY", "HTTP_PROXY", "NO_PROXY", "https_proxy", "http_proxy", "no_proxy",
];

// Non-empty so the provider counts as connected; replaced by the proxy.
const PLACEHOLDER_KEY = "injected-by-local-proxy";

function childEnvironment(root) {
  const env = {};
  for (const name of PASS_THROUGH) {
    if (process.env[name] !== undefined) env[name] = process.env[name];
  }
  // Keep the CLI's own configuration, sessions and state on the runner and out
  // of any directory that survives the job.
  env.XDG_CONFIG_HOME = scratch("config");
  env.XDG_DATA_HOME = scratch("data");
  env.XDG_STATE_HOME = scratch("state");
  env.XDG_CACHE_HOME = scratch("cache");
  // `spawnSync` changes the child's working directory but not `PWD`, and the CLI
  // resolves the project from `PWD`. Without this it reads the caller's
  // directory, finds no agent, and fails with "Agent not found".
  env.PWD = path.resolve(root);
  // Both are placeholders. The proxy replaces the auth header in flight.
  env.OPENCODE_API_KEY = PLACEHOLDER_KEY;
  env.TRIAGE_PROXY_KEY = PLACEHOLDER_KEY;
  return env;
}

// One throwaway state root per process. A stable path would let a previous run's
// project registry point the CLI at a directory that no longer exists.
let scratchRoot = null;

function scratch(name) {
  if (!scratchRoot) {
    const base = process.env.RUNNER_TEMP || process.env.TMPDIR || os.tmpdir();
    scratchRoot = mkdtempSync(path.join(base, "specta-triage-"));
  }
  const directory = path.join(scratchRoot, name);
  mkdirSync(directory, { recursive: true });
  return directory;
}

// Where the proxy forwards to: the catalog provider's endpoint unless the
// repository points `OPENCODE_BASE_URL` at another OpenAI-compatible gateway.
export function upstreamBaseUrl() {
  return (process.env.OPENCODE_BASE_URL || "https://opencode.ai/inference/openai/v1").replace(/\/+$/, "");
}

// The bare Console model id, accepting the `opencode/<id>` form used in OpenCode
// configs as well.
export function modelId() {
  return (process.env.OPENCODE_MODEL || "kimi-k3").replace(/^(opencode|triage)\//, "");
}

export function modelRef() {
  return `triage/${modelId()}`;
}

// Installs the only configuration the agent will run under.
//
// A repository config would otherwise override this one: `.opencode/opencode.json`
// outranks a root `opencode.json`, and either outranks the global config. Both
// forms are removed first, so the rules in `.github/issue-triage/opencode.json`
// cannot be loosened by anything checked into the repository.
export function prepare(root, harnessDirectory, { baseUrl } = {}) {
  for (const relative of [
    "opencode.json",
    "opencode.jsonc",
    ".opencode/opencode.json",
    ".opencode/opencode.jsonc",
    ".opencode/skills",
    ".opencode/plugins",
    ".opencode/commands",
  ]) {
    rmSync(path.join(root, relative), { recursive: true, force: true });
  }

  const config = JSON.parse(readFileSync(path.join(harnessDirectory, "opencode.json"), "utf8"));

  // A dedicated provider rather than an override of the catalog one: the catalog
  // entry refuses to route a model until it is "connected" with a real key, which
  // is exactly the key we are keeping out of this process.
  if (baseUrl) {
    config.providers = {
      triage: {
        name: "Issue triage proxy",
        env: ["TRIAGE_PROXY_KEY"],
        package: "@opencode/ai/providers/openai-compatible",
        settings: { baseURL: baseUrl },
        models: { [modelId()]: { name: modelId() } },
      },
    };
  }

  writeFileSync(path.join(root, "opencode.json"), `${JSON.stringify(config, null, 2)}\n`, {
    encoding: "utf8",
    mode: 0o644,
  });

  const agents = path.join(root, ".opencode", "agents");
  mkdirSync(agents, { recursive: true });
  for (const agent of AGENTS) {
    copyFileSync(path.join(harnessDirectory, "agents", `${agent}.md`), path.join(agents, `${agent}.md`));
  }
}

function extractFinalText(stdout) {
  const parts = [];
  for (const line of String(stdout).split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("{")) continue;

    let event;
    try {
      event = JSON.parse(trimmed);
    } catch {
      continue;
    }

    if (event?.type === "error") {
      throw new Error(`OpenCode failed: ${event.error?.message ?? "unknown error"}`);
    }

    const part = event?.part;
    if (part?.type === "text" && typeof part.text === "string") {
      parts.push({ message: part.messageID ?? "", text: part.text });
    }
  }

  if (parts.length === 0) return "";
  // Only the last assistant message is the answer; earlier ones are narration.
  const last = parts[parts.length - 1].message;
  return parts.filter((part) => part.message === last).map((part) => part.text).join("");
}

// Asynchronous on purpose: the model proxy runs in this process, so blocking the
// event loop would deadlock the CLI against a server that can never reply.
function run(root, agent, prompt, session) {
  const args = ["run", "--standalone", "--archived", "--format", "json", "--agent", agent, "--model", modelRef()];
  if (session) args.push("--session", session);
  args.push(prompt);

  return new Promise((resolve, reject) => {
    const child = spawn("opencode", args, {
      cwd: root,
      env: childEnvironment(root),
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => child.kill("SIGKILL"), RUN_TIMEOUT_MS);

    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.on("close", (status, signal) => {
      clearTimeout(timer);
      resolve({ status, signal, stdout, stderr });
    });
  });
}

async function runOnce(root, agent, prompt, session) {
  const result = await run(root, agent, prompt, session);

  const stdout = result.stdout ?? "";
  const stderr = result.stderr ?? "";

  // Parse stdout first: an error event carries a far better message than an
  // exit code, and it is how the CLI reports an unusable model or agent.
  const text = extractFinalText(stdout);

  if (result.status !== 0) {
    throw new Error(`opencode run exited ${result.status ?? "on a signal"}: ${stderr.split("\n").slice(-20).join("\n")}`);
  }

  if (!text.trim()) throw new Error(`opencode run produced no assistant text. Output tail:\n${stdout.split("\n").slice(-10).join("\n")}`);
  return text;
}

// Returns a JSON object that satisfies `schema` and `extraChecks`. The session is
// continued for the repair attempt so the model sees its own rejected answer;
// after that the caller fails closed.
export async function askJson({ root, agent, prompt, session, schema, extraChecks = () => [] }) {
  let next = prompt;
  let errors = [];

  for (let attempt = 0; attempt < 2; attempt++) {
    const reply = await runOnce(root, agent, next, session);

    let parsed;
    try {
      parsed = parseJsonLoose(reply);
    } catch (error) {
      errors = [`the reply was not valid JSON (${error.message})`];
      next = `${prompt}\n\nThat reply was not valid JSON. Reply with the JSON object only.`;
      continue;
    }

    errors = [...validate(parsed, schema), ...extraChecks(parsed)];
    if (errors.length === 0) return parsed;

    next =
      `${prompt}\n\nThe previous reply did not satisfy the schema:\n` +
      errors.map((error) => `- ${error}`).join("\n") +
      "\n\nReply with a corrected JSON object only.";
  }

  throw new Error(`model output never satisfied the schema: ${errors.join("; ")}`);
}
