# Honbaek

Honbaek is a local autonomous runtime built around six runtime concepts:

- `魂` active runtime subject
- `魄` provider-backed capability substrate
- `心` current intent and self-check state
- `身` local tools, filesystem, shell, and network boundary
- `命` durable identity and journaled continuity
- `怪異` anomalous runtime entities discovered from failures, discontinuities, and provider/tool tension

It ships as a single Rust CLI and daemon. Runtime state is local-first under `~/.honbaek/` using SQLite for structured state and JSONL for append-only history.

## Status

This project is experimental and intentionally strange. It is usable as a local CLI runtime, not a hosted service.

## Install

```bash
cargo install --git https://github.com/aproto9787/honbaek
```

From a local checkout:

```bash
cargo build --release
target/release/honbaek --help
```

## Quick Start

```bash
honbaek awaken --name default --profile unbound
honbaek assign --hon default "create a short runtime status note for this workspace"
honbaek inspect
honbaek watch
```

The daemon starts automatically when a command needs it.

## 怪異

`怪異` records anomalies in the runtime. It supports manual recording and local scans without destructive remediation.

```bash
honbaek kaeyi record "Manual omen" --evidence "operator observed unexpected runtime tension" --severity warning
honbaek kaeyi list
honbaek kaeyi inspect <id>
honbaek kaeyi contain <id> --note "held for observation"
honbaek kaeyi resolve <id> --note "explained by audit"
honbaek kaeyi scan
```

Lifecycle states are shown with Hanja labels:

- `發現` discovered
- `觀測` observed
- `封印` contained
- `解消` resolved
- `歸屬` attributed

## Provider Configuration

Honbaek includes an OpenAI-compatible provider adapter. Provider secrets are read from environment variables only and are not stored in `~/.honbaek/config.toml`.

```bash
export OPENAI_API_KEY="..."
export HONBAEK_OPENAI_MODEL="gpt-5.5"
```

Useful environment overrides:

- `HONBAEK_HOME`
- `HONBAEK_PROVIDER`
- `HONBAEK_OPENAI_BASE_URL`
- `HONBAEK_OPENAI_MODEL`
- `HONBAEK_OPENAI_API_KEY_ENV`
- `HONBAEK_OPENAI_API_KEY`

## Service And Completions

Generate a systemd user service:

```bash
honbaek service print > ~/.config/systemd/user/honbaek.service
systemctl --user enable --now honbaek.service
```

Generate shell completions:

```bash
honbaek completions bash > honbaek.bash
honbaek completions fish > honbaek.fish
honbaek completions zsh > _honbaek
```

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
scripts/smoke.sh
```

## Safety

Honbaek is local-first. `怪異` scan and containment paths observe, record, contain, and resolve local runtime state. They do not automatically delete files, kill processes, mutate external services, or make destructive network changes.

## License

MIT
