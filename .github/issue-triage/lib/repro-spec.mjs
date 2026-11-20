// The reproduction crate is generated text. It is confined to `repro/`, written
// by this program, and only ever executed in a job with no credentials.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

const MAX_FILES = 5;
const MAX_FILE_BYTES = 16 * 1024;
const MAX_TOTAL_BYTES = 48 * 1024;
const SAFE_PATH = /^repro\/(?:[A-Za-z0-9._-]+\/)*[A-Za-z0-9._-]+$/;

export function validateSpec(spec) {
  if (!spec || typeof spec !== "object") throw new Error("reproduction spec is missing");
  if (!["runtime", "compile"].includes(spec.kind)) throw new Error("reproduction kind must be runtime or compile");
  if (!Array.isArray(spec.files) || spec.files.length === 0) throw new Error("reproduction has no files");
  if (spec.files.length > MAX_FILES) throw new Error(`reproduction has more than ${MAX_FILES} files`);

  let total = 0;
  const seen = new Set();
  const files = spec.files.map((file) => {
    const filePath = String(file?.path ?? "");
    const content = String(file?.content ?? "");
    if (!SAFE_PATH.test(filePath)) throw new Error(`unsafe reproduction path ${JSON.stringify(filePath)}`);
    if (filePath.split("/").some((segment) => segment === ".." || segment === "." || segment === "")) {
      throw new Error(`unsafe reproduction path ${filePath}`);
    }
    if (seen.has(filePath)) throw new Error(`duplicate reproduction path ${filePath}`);
    seen.add(filePath);
    const bytes = Buffer.byteLength(content, "utf8");
    if (bytes > MAX_FILE_BYTES) throw new Error(`${filePath} is larger than ${MAX_FILE_BYTES} bytes`);
    total += bytes;
    return { path: filePath, content };
  });

  if (total > MAX_TOTAL_BYTES) throw new Error(`reproduction is larger than ${MAX_TOTAL_BYTES} bytes`);
  if (!seen.has("repro/Cargo.toml")) throw new Error("reproduction is missing repro/Cargo.toml");

  return { kind: spec.kind, why: String(spec.why_it_fails ?? "").slice(0, 800), files };
}

export function writeSpec(spec, root) {
  const directory = path.join(root, "repro");
  rmSync(directory, { recursive: true, force: true });
  mkdirSync(directory, { recursive: true });

  for (const file of spec.files) {
    const absolute = path.resolve(root, file.path);
    const relative = path.relative(root, absolute);
    if (relative.startsWith("..") || path.isAbsolute(relative)) throw new Error("reproduction path escaped the repository");
    mkdirSync(path.dirname(absolute), { recursive: true });
    writeFileSync(absolute, file.content, { encoding: "utf8", mode: 0o644 });
  }
}
