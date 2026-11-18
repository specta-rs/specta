// Fails when a crate declaring a native `links` key appears in specta's
// dependency tree. Cargo allows only one version of a crate per `links` value
// per build, so a supported crate that reaches one could not gain a second
// major feature without the two majors fighting over it.
//
// `cargo tree` provides the graph instead of `cargo metadata` because metadata
// lists optional dependencies that no enabled feature activates. `--target all`
// keeps platform-specific edges, such as the wasm32 paths to wasm-bindgen.

const { execFileSync } = require("child_process");
const path = require("path");

const root = path.join(__dirname, "..");
const run = (args) =>
  execFileSync("cargo", args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "inherit"],
  });

// Keyed by `links` value, not crate name: Cargo enforces uniqueness per links
// value, and differently named crates can declare the same one. `wasm_bindgen`
// is tolerated because wasm-bindgen-shared declares the link and the all-target
// graph reaches it through three independent roots: uuid (via js-sys), bson
// (via getrandom/ahash and rand), and chrono (via iana-time-zone). Chrono is too
// large to drop just to remove the exception, so it is reported as a note
// instead of failing. If a second major of one of those crates ever needs a
// different wasm-bindgen line, the exception becomes a real conflict and this
// entry must be revisited.
const allowed = new Map([
  [
    "wasm_bindgen",
    "reached on wasm targets through uuid, bson and chrono (wasm-bindgen-shared)",
  ],
]);

const tree = run([
  "tree",
  "-p",
  "specta",
  "--all-features",
  "--target",
  "all",
  "--edges",
  "normal,build",
  "--prefix",
  "none",
  "--format",
  "{p}",
]);

const metadata = JSON.parse(run(["metadata", "--format-version", "1"]));
const links = new Map(
  metadata.packages.map((pkg) => [`${pkg.name} ${pkg.version}`, pkg.links])
);

const checked = new Set();
const found = [];
for (const line of tree.split("\n")) {
  const match = /^(\S+) v(\S+)/.exec(line.trim());
  if (!match) continue;

  const id = `${match[1]} ${match[2]}`;
  if (checked.has(id)) continue;
  checked.add(id);

  const link = links.get(id);
  if (link) found.push({ id, link });
}

const unexpected = found.filter(({ link }) => !allowed.has(link));
for (const { id, link } of found.filter(({ link }) => allowed.has(link))) {
  console.warn(
    `note: ${id} declares links = "${link}" (${allowed.get(link)})`
  );
}

// Guard against a `cargo tree` format change silently parsing nothing and
// reporting success.
const hasRoot = [...checked].some((id) => id.startsWith("specta "));
if (!hasRoot || checked.size < 100) {
  console.error(
    `error: parsed ${checked.size} packages from cargo tree (specta root ${
      hasRoot ? "found" : "missing"
    }); the output format may have changed.`
  );
  process.exit(1);
}

if (unexpected.length) {
  console.error("Native `links` crates found in specta's dependency tree:");
  for (const { id, link } of unexpected)
    console.error(`  ${id} (links = "${link}")`);
  console.error(
    "Cargo allows only one version per `links` value, so a supported crate that reaches one cannot gain a second major feature. Drop the feature or add a justified entry to `allowed` in .github/links.js."
  );
  process.exit(1);
}

console.log(
  `Checked ${checked.size} packages: no unexpected native links crates.`
);
