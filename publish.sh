#!/bin/bash
# TODO: Replace this with some proper tool
# TODO: Detect if the package has changed and a release is required

set -e

cd specta-macros/
cargo publish
cd ..

cd specta/
cargo publish
cd ..

cd specta-util/
cargo publish
cd ..

cd specta-serde/
cargo publish
cd ..

cd specta-typescript/
cargo publish
cd ..

cd specta-jsdoc/
cargo publish
cd ..