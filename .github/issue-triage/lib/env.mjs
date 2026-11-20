import { appendFileSync } from "node:fs";

export function required(name) {
  const value = process.env[name];
  if (!value) {
    console.error(`::error::missing required environment variable ${name}`);
    process.exit(1);
  }
  return value;
}

export function optional(name, fallback = "") {
  return process.env[name] || fallback;
}

// Job outputs go through `$GITHUB_OUTPUT`. Values are base64 or plain single
// lines, so the heredoc form is always safe.
export function setOutput(name, value) {
  const text = String(value ?? "");
  const file = process.env.GITHUB_OUTPUT;
  if (!file) {
    console.log(`[output] ${name}=${text.slice(0, 120)}`);
    return;
  }
  appendFileSync(file, `${name}<<__SPECTA_TRIAGE_EOF__\n${text}\n__SPECTA_TRIAGE_EOF__\n`);
}

export function summary(markdown) {
  const file = process.env.GITHUB_STEP_SUMMARY;
  if (file) appendFileSync(file, markdown);
}

export function b64(text) {
  return Buffer.from(String(text ?? ""), "utf8").toString("base64");
}

export function unb64(text) {
  return Buffer.from(String(text ?? ""), "base64").toString("utf8");
}

export function truncate(text, max) {
  const value = String(text ?? "");
  if (value.length <= max) return { text: value, truncated: false };
  return { text: value.slice(0, max), truncated: true };
}
