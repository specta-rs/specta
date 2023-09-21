// This script generates the documentation for the Cargo features from the comments in the `Cargo.toml` file.
// It dumps the result into the `src/docs.md` file. This means the docs are readable in the published version of the crate and also allows the docs to be broken into a separate file (unlike `document-features`)

const fs = require("fs");
const path = require("path");

const START_MARKER = "[//]: # (FEATURE_FLAGS_START)";
const END_MARKER = "[//]: # (FEATURE_FLAGS_END)";

const docsPath = path.join(__dirname, "..", "src", "docs.md");

const cargoToml = fs.readFileSync(
  path.join(__dirname, "..", "Cargo.toml"),
  "utf8"
);
const docs = fs.readFileSync(docsPath, "utf8");

if (!docs.includes(START_MARKER) || !docs.includes(END_MARKER))
  throw new Error("Missing markers in 'docs.md'");

const lines = cargoToml.split("\n").map((line) => line.trim());

const featuresIndex = lines.indexOf("[features]");
if (featuresIndex === -1)
  throw new Error("Missing '[features]' in 'Cargo.toml'");

const featuresPart = lines.slice(featuresIndex + 1);

const endIndex = featuresPart.findIndex(
  (line) => line.startsWith("[") && line.endsWith("]")
);

const featuresLine = featuresPart.slice(0, endIndex);

let comments = "";
let group = null;
let result = {};
for (const line of featuresLine) {
  if (line == "") {
    continue;
  }

  if (line.startsWith("##!")) {
    group = "";
    result[group] = {};
  } else if (line.startsWith("##")) {
    comments += line.slice(2).trim() + "\n";
  } else if (line.startsWith("#!")) {
    group = line.substring(2).trim();
    result[group] = {};
  } else if (line.startsWith("#")) {
    continue;
  } else {
    const parts = line.split("=");
    if (parts.length !== 2) throw new Error(`Invalid feature line: '${line}'`);

    if (group !== null) {
      result[group][parts[0].trim()] = comments;
      comments = "";
    }
  }
}

let markdown_result = Object.entries(result)
  .map(
    ([name, deps]) =>
      `${name}\n\n${Object.entries(deps)
        .map(
          ([name, comment]) =>
            `- \`${name}\` - ${comment.replace("\n", " ").trim()}`
        )
        .join("\n")}`
  )
  .join("\n\n");

const resultDocs = docs.replace(
  docs.substring(
    docs.indexOf(START_MARKER),
    docs.lastIndexOf(END_MARKER) + END_MARKER.length
  ),
  START_MARKER + "\n" + markdown_result + "\n\n" + END_MARKER
);

fs.writeFileSync(docsPath, resultDocs);
