#!/usr/bin/env bash
set -euo pipefail

export HONBAEK_OPENAI_API_KEY_ENV="${HONBAEK_OPENAI_API_KEY_ENV:-HONBAEK_SMOKE_NO_KEY}"

cargo build
cargo run -- daemon stop >/tmp/honbaek-daemon-stop.out 2>/tmp/honbaek-daemon-stop.err || true
cargo run -- awaken
cargo run -- assign "create a short README status note for this workspace"
gyeryeong_warn_pattern="gyeryeong-warn-smoke-$$"
cargo run -- gyeryeong add "Warn smoke prompt" --pattern "$gyeryeong_warn_pattern" --action warn --rationale "operator review required" >/tmp/honbaek-gyeryeong-warn.out
gyeryeong_warn_id="$(awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^[0-9a-fA-F-]{36}$/) { print $i; exit } }' /tmp/honbaek-gyeryeong-warn.out)"
test -n "$gyeryeong_warn_id"
cargo run -- gyeryeong list
cargo run -- gyeryeong inspect "$gyeryeong_warn_id"
cargo run -- assign "$gyeryeong_warn_pattern create a harmless status note"
gyeryeong_block_pattern="forbidden-gyeryeong-smoke-$$"
cargo run -- gyeryeong add "Block smoke prompt" --pattern "$gyeryeong_block_pattern" --action block --rationale "blocking path smoke" >/tmp/honbaek-gyeryeong-block.out
gyeryeong_block_id="$(awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^[0-9a-fA-F-]{36}$/) { print $i; exit } }' /tmp/honbaek-gyeryeong-block.out)"
test -n "$gyeryeong_block_id"
if cargo run -- assign "$gyeryeong_block_pattern"; then
  echo "expected 戒令 block smoke to fail" >&2
  exit 1
fi
cargo run -- gyeryeong disable "$gyeryeong_block_id"
cargo run -- assign "$gyeryeong_block_pattern"
cargo run -- gyeryeong enable "$gyeryeong_block_id"
cargo run -- kaeyi record "Manual omen" --evidence "operator observed unexpected runtime tension" --severity warning >/tmp/honbaek-kaeyi-record.out
kaeyi_id="$(awk '/怪異/ { print $2; exit }' /tmp/honbaek-kaeyi-record.out)"
cargo run -- kaeyi list
cargo run -- kaeyi inspect "$kaeyi_id"
cargo run -- kaeyi contain "$kaeyi_id" --note "held for observation"
cargo run -- kaeyi resolve "$kaeyi_id" --note "explained by smoke audit"
cargo run -- kaeyi scan
cargo run -- inspect
cargo run -- watch --once
cargo run -- service print >/tmp/honbaek.service
cargo run -- completions bash >/tmp/honbaek.bash

test -s README.md
test -s HONBAEK_STATUS.md
test -s "$HOME/.honbaek/state.sqlite3"
test -s "$HOME/.honbaek/journal.jsonl"
test -s /tmp/honbaek.service
test -s /tmp/honbaek.bash
