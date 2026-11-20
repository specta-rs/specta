// Read-only repository access for the model. Paths are resolved against the
// clone, symlinks are rejected, binaries and build output are unreadable.

import { readFileSync, readdirSync, realpathSync, statSync } from "node:fs";
import path from "node:path";

const DENY_SEGMENTS = new Set([".git", "target", "node_modules", "repro", "vendor", ".venv"]);
const TEXT_EXTENSIONS = new Set([
  ".rs", ".toml", ".md", ".json", ".jsonc", ".yml", ".yaml", ".ts", ".tsx",
  ".js", ".mjs", ".txt", ".snap", ".lock", ".css", ".html", ".sh",
]);
const MAX_FILE_BYTES = 256 * 1024;
const MAX_READ_CHARS = 12_000;
const MAX_SEARCH_MATCHES = 40;

export function createRepoTools(root) {
  const rootReal = realpathSync(root);

  function resolve(input) {
    const raw = String(input ?? ".");
    if (raw.includes("\u0000")) throw new Error("invalid path");
    const absolute = path.resolve(rootReal, raw);
    const relative = path.relative(rootReal, absolute);
    if (relative.startsWith("..") || path.isAbsolute(relative)) throw new Error("path is outside the repository");
    if (relative.split(path.sep).some((segment) => DENY_SEGMENTS.has(segment))) {
      throw new Error("path is not readable");
    }
    return { absolute, relative: relative || "." };
  }

  return {
    tools: [
      {
        type: "function",
        function: {
          name: "read_file",
          description: "Read a UTF-8 text file from the repository. Returns at most 400 lines.",
          parameters: {
            type: "object",
            properties: {
              path: { type: "string", description: "Repository-relative path, for example specta/src/lib.rs" },
              offset: { type: "integer", description: "First line to return, 1-based" },
              limit: { type: "integer", description: "Maximum number of lines to return" },
            },
            required: ["path"],
            additionalProperties: false,
          },
        },
      },
      {
        type: "function",
        function: {
          name: "list_dir",
          description: "List the entries of a repository directory.",
          parameters: {
            type: "object",
            properties: { path: { type: "string", description: "Repository-relative directory" } },
            required: ["path"],
            additionalProperties: false,
          },
        },
      },
      {
        type: "function",
        function: {
          name: "search_repo",
          description: "Case-insensitive substring search across repository source files. Returns path:line matches.",
          parameters: {
            type: "object",
            properties: {
              query: { type: "string", description: "Substring to look for, at least 3 characters" },
              limit: { type: "integer", description: "Maximum number of matches" },
            },
            required: ["query"],
            additionalProperties: false,
          },
        },
      },
    ],
    handlers: {
      read_file({ path: inputPath, offset = 1, limit = 400 }) {
        const { absolute, relative } = resolve(inputPath);
        const stats = statSync(absolute);
        if (!stats.isFile()) throw new Error("not a file");
        if (!TEXT_EXTENSIONS.has(path.extname(absolute).toLowerCase())) throw new Error("not a text file");
        if (stats.size > MAX_FILE_BYTES) throw new Error(`file is larger than ${MAX_FILE_BYTES} bytes`);

        const lines = readFileSync(absolute, "utf8").split("\n");
        const start = Math.max(1, Number(offset) || 1);
        const count = Math.min(400, Math.max(1, Number(limit) || 400));
        const slice = lines.slice(start - 1, start - 1 + count);
        const body = slice.map((line, index) => `${start + index}\t${line}`).join("\n");
        return `${relative} (${lines.length} lines total)\n${body.slice(0, MAX_READ_CHARS)}`;
      },

      list_dir({ path: inputPath = "." }) {
        const { absolute, relative } = resolve(inputPath);
        if (!statSync(absolute).isDirectory()) throw new Error("not a directory");
        const entries = readdirSync(absolute, { withFileTypes: true })
          .filter((entry) => !DENY_SEGMENTS.has(entry.name))
          .slice(0, 300)
          .map((entry) => `${entry.isDirectory() ? "dir " : "file"} ${entry.name}`);
        return `${relative}\n${entries.join("\n")}`;
      },

      search_repo({ query, limit = 20 }) {
        const needle = String(query ?? "").toLowerCase();
        if (needle.length < 3) throw new Error("query must be at least 3 characters");
        const max = Math.min(MAX_SEARCH_MATCHES, Math.max(1, Number(limit) || 20));
        const matches = [];
        let scanned = 0;

        const walk = (directory) => {
          if (matches.length >= max || scanned > 20_000) return;
          for (const entry of readdirSync(directory, { withFileTypes: true })) {
            if (matches.length >= max || scanned > 20_000) return;
            if (DENY_SEGMENTS.has(entry.name) || entry.name.startsWith(".git")) continue;
            const absolute = path.join(directory, entry.name);
            if (entry.isDirectory()) {
              walk(absolute);
              continue;
            }
            if (!entry.isFile() || !TEXT_EXTENSIONS.has(path.extname(entry.name).toLowerCase())) continue;
            scanned += 1;
            const stats = statSync(absolute);
            if (stats.size > MAX_FILE_BYTES) continue;
            const lines = readFileSync(absolute, "utf8").split("\n");
            for (let index = 0; index < lines.length && matches.length < max; index++) {
              if (lines[index].toLowerCase().includes(needle)) {
                const relative = path.relative(rootReal, absolute);
                matches.push(`${relative}:${index + 1}: ${lines[index].trim().slice(0, 240)}`);
              }
            }
          }
        };

        walk(rootReal);
        return matches.length ? matches.join("\n") : `no matches for "${needle}"`;
      },
    },
  };
}
