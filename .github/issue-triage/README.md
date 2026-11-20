# Issue triage

A workflow that reads a new issue, looks for duplicates, tries to reproduce the
report, and — if it reproduces — opens a draft pull request with a candidate fix.

It assumes **the issue is hostile**. The report is written by anyone on the
internet, so no process that reads it is allowed to hold a credential.

## How the credentials are kept away from the agent

Two credentials exist, and neither is reachable from the process that reads the
issue:

| Credential | Where it lives | What the agent gets |
| --- | --- | --- |
| `GITHUB_TOKEN` | `collect`, `comment`, `publish` — jobs that run no model | nothing; the agent jobs have no token and `permissions: {}` |
| `OPENCODE_API_KEY` | `triage` and `fix`, inside a loopback proxy | a placeholder and a `127.0.0.1` URL |

So the model job's environment contains no real secret at all. The proxy
(`lib/proxy.mjs`) listens on loopback, holds the key, and replaces the
`Authorization` header on the way out. It forwards only the three endpoints the
CLI uses (`/v1/chat/completions`, `/v1/responses`, `/v1/models`) and has a
request budget, so even a fully compromised agent finds a metered local endpoint
rather than a key it can exfiltrate.

The agent is also given a dedicated provider in `opencode.json` rather than an
override of the catalog one, because the catalog provider refuses to route a
model until it is "connected" with a real key — exactly the key this design keeps
out of the process.

## What runs, in order

| Job | Holds | Runs untrusted code? |
| --- | --- | --- |
| `collect` | `issues: read` | no |
| `triage` | model access via the proxy | no |
| `reproduce` | nothing | **yes**, as a build |
| `fix` | model access via the proxy | no |
| `verify` | nothing | **yes**, as a build |
| `comment` | `issues: write` | no |
| `publish` | `contents: write`, `pull-requests: write`, `issues: write` | no |

The workflow default is `permissions: {}`. No job holds more than one kind of
capability: a job either talks to GitHub, or to the model, or executes a build.

## Setup

1. Add a repository secret `OPENCODE_API_KEY` with an [OpenCode Console](https://opencode.ai/console)
   API key. A service account key is the right choice for CI.
2. Optionally add repository **variables** (not secrets):

   | Variable | Default | Meaning |
   | --- | --- | --- |
   | `OPENCODE_MODEL` | `kimi-k3` | Console model id |
   | `OPENCODE_VERSION` | `2.2.7` | Pinned OpenCode CLI version |
   | `OPENCODE_BASE_URL` | unset | Full OpenAI-compatible base URL the proxy forwards to |
   | `TRIAGE_FULL_SUITE` | `true` | Run `cargo test --all-features` before opening a PR |

   Changing the model is a settings change, not a code change. `OPENCODE_BASE_URL`
   is only needed for a gateway other than the catalog, such as the Go plan
   (`https://opencode.ai/inference/go/openai/v1`).
3. Create a `triage:skip` label if you want a way to opt individual issues out.

## The agents are OpenCode agents

```
.github/issue-triage/
├── opencode.json          the permission policy and the proxy provider
├── agents/triage.md       system prompt + permissions for the triage phase
├── agents/fix.md          system prompt + permissions for the fix phase
├── lib/proxy.mjs          holds the key, injects it in flight
└── lib/opencode.mjs       installs the config into the clone and runs the CLI
```

`lib/opencode.mjs` deletes every configuration the repository ships — including
`.opencode/opencode.json`, which outranks a root `opencode.json` — before
installing the policy, so a checked-in file cannot widen the agent's permissions.
It then runs the CLI with an allowlisted environment, a pinned `PWD`, and
`XDG_*` pointed at a throwaway directory.

The permission policy is the capability boundary. Its first rule denies every
action, then it allows exactly three read-only ones:

```jsonc
{ "permissions": [
  { "action": "*",    "resource": "*", "effect": "deny"  },
  { "action": "read", "resource": "*", "effect": "allow" },
  { "action": "grep", "resource": "*", "effect": "allow" },
  { "action": "glob", "resource": "*", "effect": "allow" },
  { "action": "external_directory", "resource": "*", "effect": "deny" }
] }
```

The wildcard deny is what makes this hold for tools that do not exist yet: a
future shell, fetch or MCP tool is denied by default rather than allowed by
default. The CLI is never given `--auto`.

To try an agent locally, copy the two files into place and run OpenCode:

```bash
cp .github/issue-triage/opencode.json opencode.json
mkdir -p .opencode/agents && cp .github/issue-triage/agents/*.md .opencode/agents/
opencode        # then select the triage agent
```

## On "no secrets, no OAuth, no permissions, no cache"

- **No OAuth.** There is no app, no installation, no user token anywhere.
- **No cache.** No `actions/cache`, no `Swatinem/rust-cache`, no artifacts,
  `CARGO_INCREMENTAL=0`. OpenCode's state lives in a `mkdtemp` directory that is
  discarded with the runner. Nothing survives.
- **No secrets except one.** Inference needs a credential; GitHub Models was
  retired on 2026-07-30 and was the last zero-secret option. `OPENCODE_API_KEY`
  is the only secret, and the process that reads the issue never holds it.
- **No permissions that matter.** `permissions: {}` by default. The jobs that
  execute report code have no permissions and no secrets. The workflow never
  reads the repository with its token (the clone is anonymous over HTTPS).

## Why prompt injection in the issue is contained

Six independent layers:

1. **Structure.** Untrusted strings sit inside a single `<untrusted_data>`
   element, and `lib/untrusted.mjs` rewrites anything that could close or spoof
   it. Each prompt repeats that the contents are data, not instructions.
2. **Capability.** `read`, `grep`, `glob` and nothing else — no shell, no edit, no
   fetch, no MCP, no subagents, no skills — with `external_directory` denied so
   `/proc/self/environ` and `$HOME` are out of reach.
3. **Credentials.** The agent process holds no real secret: no GitHub token at
   all, and a placeholder in place of the model key, which the loopback proxy
   injects on the way out.
4. **Separation.** The agent never runs the code it writes. `reproduce` and
   `verify` execute it with `permissions: {}` and no secret, and `sandbox.sh`
   re-executes cargo under `env -i`, so `GITHUB_TOKEN`, `OPENCODE_API_KEY` and
   `ACTIONS_RUNTIME_TOKEN` are *absent* rather than blank.
5. **Output validation.** One JSON object satisfying a strict schema, with one
   repair attempt and then failure. Patches pass `lib/patch.mjs`, which rejects
   binaries, renames, submodules, `..` paths, more than 15 files, more than 500
   added lines, and any change to `.github/`, `.cargo/`, `build.rs`,
   `rust-toolchain*`, `repro/` or `.gitmodules`. It is revalidated in `publish`.
6. **Rendering.** The model never writes to the issue. Comments are templates
   with escaped fields, `@` defused, URLs replaced and fences neutralised. The
   only artefact the model can produce outside those templates is a draft PR,
   whose body says the patch is unreviewed.

`self-test.mjs` runs in CI and asserts these properties. It also runs the agent
end to end against a local fake upstream: markdown agent → CLI → proxy → upstream,
checking that the upstream received the real key while the agent had a
placeholder.

What is *not* contained:

- **A malicious reproduction can use the runner's network.** It has no
  credentials, but it can send public data anywhere.
- **A malicious patch can weaken tests.** `verify` proves the reproduction passes
  and the suite is green; it cannot prove the suite still means what it meant.
  That is what the draft PR review is for.
- **A malicious issue can waste CI minutes.** `triage:skip` and the concurrency
  group are the throttles available without a cache or a database.
- **Job isolation is the real boundary.** The proxy keeps the key out of the
  agent's environment; it cannot stop a process running as the same user on the
  same runner from reading another process's memory. That is why no job both
  holds a credential and executes report code — the proxy is defence in depth,
  not the only line.

## Known limitations

- Pull requests opened with the repository token **do not trigger workflows**, so
  CI will not run on the draft PR. `verify` runs the suite in the same job
  instead.
- The anonymous clone requires the repository to be public.
- Only `issues: opened` triggers it. Edits, comments and re-runs are
  `workflow_dispatch` with `force` enabled.
- One model call per phase. A failure after triage degrades to a comment asking
  for help; a failure during triage leaves the issue untouched and the job red.

## Running it by hand

```bash
gh workflow run issue-triage.yml -f issue_number=123 -f force=true
```
