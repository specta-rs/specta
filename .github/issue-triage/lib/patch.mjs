// A model-produced diff is untrusted input to `git apply`. Validate it before
// it is stored, revalidate it before it is applied, and never let it reach CI,
// the build system, or a credential.

const DENIED_PATHS = [
  /^\.github\//,        // CI and this workflow itself
  /^\.cargo\//,         // registry and source replacement
  /^\.git/,             // hooks, config, modules
  /^build\.rs$/,        // arbitrary code at build time
  /^rust-toolchain/,    // toolchain pinning
  /^\.gitmodules$/,
  /^repro\//,           // the scratch reproduction harness
];

export const MAX_PATCH_BYTES = 96 * 1024;
const MAX_ADDED_LINES = 500;
const MAX_FILES = 15;

export function inspectPatch(patch) {
  if (typeof patch !== "string" || patch.trim() === "") throw new Error("patch is empty");
  if (patch.length > MAX_PATCH_BYTES) throw new Error(`patch is larger than ${MAX_PATCH_BYTES} bytes`);
  if (patch.includes("\u0000")) throw new Error("patch contains a NUL byte");
  if (/^GIT binary patch$/m.test(patch) || /^Binary files /m.test(patch)) throw new Error("binary patches are not allowed");
  if (/^Subproject commit /m.test(patch)) throw new Error("submodule changes are not allowed");

  const files = [];
  for (const line of patch.split("\n")) {
    const match = /^diff --git a\/(\S+) b\/(\S+)$/.exec(line);
    if (!match) continue;
    const [, from, to] = match;
    if (from !== to) throw new Error(`renaming ${from} is not allowed`);
    files.push(to);
  }

  if (files.length === 0) throw new Error("patch does not contain any 'diff --git' hunks");
  if (files.length > MAX_FILES) throw new Error(`patch touches ${files.length} files, limit is ${MAX_FILES}`);

  const seen = new Set();
  for (const file of files) {
    if (seen.has(file)) throw new Error(`${file} appears twice in the patch`);
    seen.add(file);
    if (file.startsWith("/") || file.includes("..") || file.includes("\\")) throw new Error(`unsafe path ${file}`);
    if (DENIED_PATHS.some((pattern) => pattern.test(file))) throw new Error(`patch may not touch ${file}`);
  }

  const added = patch.split("\n").filter((line) => line.startsWith("+") && !line.startsWith("+++")).length;
  if (added > MAX_ADDED_LINES) throw new Error(`patch adds ${added} lines, limit is ${MAX_ADDED_LINES}`);

  return { files, added };
}

