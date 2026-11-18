// Fails when a crate declaring a native `links` key appears in specta's
// dependency tree. Cargo allows only one version of a crate per `links` value
// per build, so a supported crate that reaches one could not gain a second
// major feature without the two majors fighting over it.
//
// `cargo tree` provides the graph instead of `cargo metadata` because metadata
// lists optional dependencies that no enabled feature activates. `--target all`
// keeps platform-specific edges, such as chrono -> wasm-bindgen on wasm32.

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
// is unavoidable while chrono's wasm32 bindings are supported: chrono enables
// wasm-bindgen, whose shared crate declares the link. Coexisting chrono majors
// remain possible as long as they resolve the same wasm-bindgen line.
const allowed = new Map([
  [
    "wasm_bindgen",
    "reached through chrono's wasm32 support (wasm-bindgen-shared)",
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
  "--no-dedupe",
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
