// Every comment on an issue is rendered here, from fixed prose plus escaped
// fields. The model never writes to the issue directly, so it cannot mention
// users, link domains, or impersonate a maintainer.

import { escapeModelText } from "./untrusted.mjs";

export const MARKER = "<!-- specta-issue-triage -->";

function render({ repo, number, body }) {
  return [
    MARKER,
    body,
    "",
    "<sub>Automated by `.github/workflows/issue-triage.yml`. No credentials were " +
      "given to the code in this report; a maintainer still decides what happens next.</sub>",
    `<sub>[How this works](https://github.com/${repo}/blob/main/.github/issue-triage/README.md)</sub>`,
  ].join("\n");
}

export function duplicateComment({ repo, number, duplicateOf, confidence, summary }) {
  return render({
    repo,
    number,
    body: [
      `### Possible duplicate of #${duplicateOf}`,
      "",
      `This looks like it describes the same problem as #${duplicateOf} (${confidence}% confidence).`,
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      "If that is a different problem, reply here and a maintainer will take another look. " +
        "Nothing was closed automatically.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}

export function needsInfoComment({ repo, number, missing, summary }) {
  const items = (missing ?? []).slice(0, 6).map((item) => `- ${escapeModelText(item, { max: 160 })}`);
  return render({
    repo,
    number,
    body: [
      "### We need a bit more to go on",
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      "To investigate this we need a report we can run. The most useful thing you can add is a **minimal, self-contained reproduction** — the smallest program that shows the problem.",
      "",
      items.length ? "Still missing:" : "Useful details:",
      ...items,
      "",
      "<details><summary>What a good reproduction looks like</summary>",
      "",
      "```toml",
      "[dependencies]",
      `specta = { version = "2", path = "../specta" }`,
      `specta-typescript = { version = "0.0.10", path = "../specta-typescript" }`,
      "```",
      "",
      "```rust",
      "use specta::Type;",
      "",
      "#[derive(Type)]",
      "struct Example {",
      "    value: u32,",
      "}",
      "",
      "fn main() {",
      "    // what you expected, and what actually happened",
      "}",
      "```",
      "",
      "</details>",
      "",
      "Please include the exact `cargo` command, the full error output, and the versions in your `Cargo.lock`. " +
        "Without that we can only guess, and a wrong guess wastes your time.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}

export function notABugComment({ repo, number, summary }) {
  return render({
    repo,
    number,
    body: [
      "### This looks like expected behaviour",
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      "If you think this should work differently, reply with the behaviour you expected and why — " +
        "that is a design discussion a maintainer should have with you rather than something to fix quietly.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}

export function cannotReproduceComment({ repo, number, kind, log, summary }) {
  const excerpt = String(log ?? "")
    .split("\n")
    .slice(-40)
    .join("\n")
    .slice(0, 2_500)
    // The log is controlled by the reproduction, which is model output, which is
    // derived from a public issue. It must not be able to close the fence.
    .replace(/`{3,}/g, "``'")
    .replace(/<!--/g, "< !--")
    .replace(/@/g, "@\u200b");
  return render({
    repo,
    number,
    body: [
      "### We could not reproduce this",
      "",
      `A sandboxed job built the reproduction from this report and ran it, but it did not fail in the way described (\`${kind}\`).`,
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      excerpt
        ? ["<details><summary>Last lines of the run</summary>", "", "```text", excerpt, "```", "", "</details>"].join("\n")
        : "",
      "",
      "This usually means the report is missing a piece of setup, a feature flag, or a version. " +
        "If you can paste a complete `main.rs` and `Cargo.toml` that still fails on the latest `main`, " +
        "a maintainer can pick it up from there.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}

export function fixFailedComment({ repo, number, runUrl, summary }) {
  return render({
    repo,
    number,
    body: [
      "### Reproduced, but no fix yet",
      "",
      "A sandboxed job reproduced this, so it is a real defect. The automated fix step did not produce a patch that survived review by the workflow's own guards, and nothing was opened.",
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      runUrl ? `[Run details](${runUrl})` : "",
      "",
      "A maintainer needs to take this one. The reproduction above is a good starting point.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}

export function verifyFailedComment({ repo, number, runUrl, summary }) {
  return render({
    repo,
    number,
    body: [
      "### Reproduced, but the candidate fix did not verify",
      "",
      "A sandboxed job applied a generated patch and reran the reproduction and the workspace test suite. It did not come back green, so no pull request was opened.",
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      runUrl ? `[Run details](${runUrl})` : "",
      "",
      "The reproduction in this thread still stands, so this is worth a maintainer's time.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}

export function fixedComment({ repo, number, pull, summary }) {
  return render({
    repo,
    number,
    body: [
      `### Draft fix opened: #${pull}`,
      "",
      "A sandboxed job reproduced this, and a draft pull request with a candidate fix is open.",
      "",
      summary ? `> ${escapeModelText(summary, { max: 500 })}` : "",
      "",
      "The fix was written by a model and has **not** been reviewed. It is a starting point, not an answer.",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  });
}
