#!/usr/bin/env bash
# Runs one command with an empty environment.
#
# The reproduction is untrusted code. It must not be able to read the job's
# credentials, so this wrapper throws away everything except the handful of
# variables a Rust build actually needs. GITHUB_TOKEN, OPENCODE_API_KEY and
# ACTIONS_RUNTIME_TOKEN are all absent from the child, not blanked.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: sandbox.sh <command> [args...]" >&2
  exit 64
fi

sandbox_home="${RUNNER_TEMP:-/tmp}/specta-triage-home"
mkdir -p "$sandbox_home"

exec env -i \
  PATH="$PATH" \
  HOME="$sandbox_home" \
  TMPDIR="${TMPDIR:-/tmp}" \
  CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
  RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" \
  CARGO_TERM_COLOR=never \
  CARGO_INCREMENTAL=0 \
  CARGO_NET_RETRY=2 \
  RUST_BACKTRACE=0 \
  "$@"
