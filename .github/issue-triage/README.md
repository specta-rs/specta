# Issue triage

A workflow that reads a new issue, looks for duplicates, tries to reproduce the
report, and — if it reproduces — opens a draft pull request with a candidate fix.

It is built on the assumption that **the issue is hostile**. The report is
written by anyone on the internet; it is never trusted, and it is never given to
anything that holds a credential it could exfiltrate.

## What runs, in order

| Job | What it does | Executes code from the issue? | Permissions |
| --- | --- | --- | --- |
| `triage` | Finds duplicates, decides if the report is actionable, writes a reproduction crate | no | `issues: write` |
| `reproduce` | Builds and runs that crate | **yes** | **none** |
| `fix` | Asks the model for a patch | no | **none** |
| `verify` | Applies the patch, runs the reproduction and `cargo test --all-features` | **yes** | **none** |
| `ask-for-help` | Comments when nothing was reproduced or no safe patch came out | no | `issues: write` |
| `publish` | Pushes a branch and opens a draft pull request | no | `contents: write`, `pull-requests: write`, `issues: write` |

The workflow default is `permissions: {}`. Every job opts in to the smallest set
it needs, and **no job both holds a credential and executes code from the
report**: the two jobs that run untrusted code have an empty permission set and
are given no secrets.

## Setup

1. Add a repository secret `OPENCODE_API_KEY` with an [OpenCode Console](https://opencode.ai/console)
   API key. A service account key is the right choice for CI; it is not tied to
   one person.
2. Optionally add repository **variables** (not secrets) to pick the model:

   | Variable | Default | Meaning |
   | --- | --- | --- |
   | `OPENCODE_MODEL` | `kimi-k3` | Model id served by Console |
   | `OPENCODE_BASE_URL` | `https://opencode.ai/inference` | Console gateway; use the `/go` base for a Go plan |
   | `TRIAGE_FULL_SUITE` | `true` | Run `cargo test --all-features` before opening a PR |

   Use the bare model id from the Console catalog (`kimi-k3`, `glm-5.3`,
   `claude-sonnet-5-5`); an `opencode/` prefix is accepted and stripped.
   Changing the model is therefore a settings change, not a code change.
3. Create a `triage:skip` label if you want a way to opt individual issues out.

## On "no secrets, no OAuth, no permissions, no cache"

Some of that is achievable and some of it is not. Precisely:

- **No OAuth.** There is no app, no installation, no user token anywhere.
- **No cache.** No `actions/cache`, no `Swatinem/rust-cache`, no uploaded
  artifacts. `CARGO_INCREMENTAL=0` and nothing survives the runner.
- **No secrets except one.** An LLM has to authenticate somewhere. GitHub Models
  was retired on 2026-07-30 and was the only zero-secret inference path in
  Actions, so inference now requires `OPENCODE_API_KEY`. It is the only secret
  the workflow reads, and only the `triage` and `fix` jobs receive it. A truly
  secret-free workflow can only be deterministic, and then it cannot write a fix.
- **No permissions that matter.** `permissions: {}` is the default. The jobs that
  touch untrusted code have no permissions at all. The workflow can never read
  repository contents with its token (it clones anonymously over HTTPS) and the
  only job that can push is `publish`, which runs nothing from the report.

## Why prompt injection in the issue is contained

Five independent layers, in the order an attacker would have to defeat them:

1. **Structure.** Every third-party string sits inside a single
   `<untrusted_data>` element. `lib/untrusted.mjs` strips control characters
   and rewrites `<untrusted_data>` / `</untrusted_data>` (and the other wrapper
   names) so the text cannot close its own container, and each prompt repeats
   that the contents are data, not instructions.
2. **Capability.** The model has exactly three tools — `read_file`, `list_dir`,
   `search_repo` — implemented in `lib/fs-tools.mjs`. There is no shell, no
   network tool, no write tool, and no way to ask the workflow to perform an
   action. The strongest thing a successful injection can do is make the model
   read files it would have read anyway.
3. **Separation.** The agent never runs the code it writes. `reproduce` and
   `verify` execute it in jobs with `permissions: {}` and no secret, and
   `sandbox.sh` re-executes cargo under `env -i`, so `GITHUB_TOKEN`,
   `OPENCODE_API_KEY` and `ACTIONS_RUNTIME_TOKEN` are *absent* rather than blank.
4. **Output validation.** The model must emit one JSON object that satisfies a
   strict schema (`lib/schema.mjs`): unknown fields, wrong types, out-of-range
   integers and over-long strings are rejected, with one repair attempt and then
   failure. Patches additionally pass `lib/patch.mjs`, which rejects binary
   diffs, renames, submodules, `..` paths, more than 15 files, more than 500
   added lines, and any change to `.github/`, `.cargo/`, `build.rs`,
   `rust-toolchain*`, `repro/` or `.gitmodules`. The patch is revalidated in
   `publish`, not trusted from the previous job.
5. **Rendering.** The model does not write to the issue. Comments are templates
   in `lib/comment.mjs`: fixed prose, escaped model fields, `@` defused,
   URLs defused, links constructed by the workflow. `publish` only gets a
   branch and a draft PR, and the PR body says plainly that the patch is
   unreviewed.

What is *not* contained, and should be understood before enabling this:

- **A malicious reproduction can use the runner's network.** It has no
  credentials, but it can send the patch and the report to any host. Treat the
  work as public; it came from a public issue anyway.
- **A malicious patch can weaken tests.** `verify` proves the reproduction
  passes and the suite is green; it cannot prove the suite still means what it
  meant. That is what the human review of the draft PR is for.
- **A malicious issue can waste CI minutes.** Duplicate reports cost one model
  call; actionable ones cost a build. `triage:skip` and the concurrency group
  are the throttles available without a cache or a database.

## Known limitations

- Pull requests opened with the repository token **do not trigger workflows**, so
  CI will not run on the draft PR. `verify` runs the suite in the same job
  instead. If you want CI on the PR, the only way is a personal access token,
  which is exactly the credential this design avoids.
- The anonymous clone requires the repository to be public.
- Only `issues: opened` triggers it. Edits, comments and re-runs are
  `workflow_dispatch` with `force` enabled.
- One model call per phase, no retries across phases. A failure after triage
  degrades to a comment asking for help. A failure during triage leaves the issue
  untouched and the job red, so it is visible in the Actions tab rather than
  silently guessed at.

## Running it by hand

```bash
gh workflow run issue-triage.yml -f issue_number=123 -f force=true
```

To exercise a single stage locally:

```bash
OPENCODE_API_KEY=oc_sk_... \
OPENCODE_MODEL=kimi-k3 \
GITHUB_TOKEN="$(gh auth token)" \
REPO=specta-rs/specta \
ISSUE_NUMBER=123 \
node .github/issue-triage/triage.mjs .
```
