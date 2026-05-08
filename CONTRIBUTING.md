# Contributing

Honbaek is a local-first Rust runtime. Contributions should keep the runtime inspectable, conservative around side effects, and clear about the boundary between local state and provider calls.

## Development Setup

```bash
cargo build
cargo test --workspace
```

Run the full local verification set before opening a pull request:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
scripts/smoke.sh
```

## Design Rules

- Keep runtime state local by default.
- Do not persist provider secrets in config files or journals.
- Avoid destructive remediation in `怪異` paths.
- Prefer small modules with direct ownership over broad abstractions.
- Make CLI behavior observable through `inspect`, `watch --once`, or journaled events.

## Pull Requests

Pull requests should include:

- What changed.
- Why the runtime needed the change.
- The commands used for verification.
- Any user-visible CLI, TUI, storage, or provider behavior changes.

Code artifacts in this repository are written in English. Runtime concepts may use Hanja where they are part of the product surface.
