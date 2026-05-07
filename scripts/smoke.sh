#!/usr/bin/env bash
set -euo pipefail

export HONBAEK_OPENAI_API_KEY_ENV="${HONBAEK_OPENAI_API_KEY_ENV:-HONBAEK_SMOKE_NO_KEY}"

cargo build
cargo run -- awaken
cargo run -- assign "create a short README status note for this workspace"
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
