# 혼백강령 Product Spec

## Source Request

`$interview-heddle 혼백강령`

The product idea is a local autonomous runtime where LLM provider adapters act as `魄` and a persistent runtime organizes `魂`: agency, memory, goals, self-inspection, and action.

## Goal

Build 혼백강령 as a product-grade local autonomous runtime, not an MVP or demo. The result must provide a Rust CLI, a daemon, persistent local state, observable autonomous execution, and multi-`魂` orchestration that can perform real local repository work end to end.

## Current Context

- Target workspace is empty: `/home/argoss/dev/das1`.
- The workspace is not currently a git repository.
- No project-local `AGENTS.md`, README, or build manifest exists.
- Global instructions require Korean user-facing reports, English code artifacts where applicable, clean modular implementation, and usable verification beyond tests.

## Scope

- Create a Rust project for a single local product named `honbaek`.
- Provide a CLI with these primary commands:
  - `honbaek awaken`
  - `honbaek assign "<task>"`
  - `honbaek watch`
  - `honbaek inspect`
- Provide a local daemon that can maintain persistent runtime state across commands.
- Model the core domain around the important concepts with Hanja labels:
  - `魂`: active runtime subject / autonomous agent instance.
  - `魄`: provider-backed capability body / model adapter substrate.
  - `心`: current priority, tension, intent, and self-check state.
  - `身`: local tools, filesystem, process, shell, and network interface.
  - `命`: durable identity, history, commitments, and continuity.
- Implement an OpenAI-compatible provider adapter as the first completed provider.
- Keep the provider layer extensible for additional provider adapters without hard-coding the runtime to one API.
- Implement product-grade autonomy around explicit `unbound` profiles:
  - file creation and modification
  - file deletion
  - command execution
  - network requests
- Implement multi-`魂` orchestration as a first-class product capability, not a future-only note.
- Store local state under `~/.honbaek/`:
  - SQLite for structured state
  - JSONL journal for append-only runtime history
  - `config.toml` for non-secret configuration
- Support environment-variable overrides for provider secrets and sensitive settings. Secrets must not be stored directly in `config.toml`.
- Provide `honbaek watch` as a TUI-grade real-time view.
- Provide `honbaek inspect` for structured current-state inspection.
- Include install/runtime polish:
  - single Rust binary
  - systemd user service support
  - shell completion generation or installation path

## Non-Goals

- Do not reduce the product to a minimal chat wrapper.
- Do not stop at a philosophical simulation without real filesystem/command effects.
- Do not treat multi-`魂` orchestration as out of scope.
- Do not store provider API keys or secrets directly in `~/.honbaek/config.toml`.
- Do not silently execute irreversible external side effects during verification. Product support for unbound operation must exist, but destructive external actions still require explicit human approval when run from this Codex session.
- Do not implement a web dashboard unless the CLI/TUI/runtime product is complete and acceptance criteria already pass.

## Decisions

- Product scope is product-grade from the start; internal phases are allowed, but final acceptance is not MVP-grade.
- Implementation stack is Rust.
- Primary interface is CLI plus TUI `watch`.
- Runtime shape is local CLI plus daemon.
- First core behavior is autonomous task execution.
- First practical task domain is local repository work.
- Provider scope is OpenAI-compatible adapter completed first, with a clean adapter boundary for future providers.
- Autonomy model uses explicit `unbound` profiles.
- State location is `~/.honbaek/`.
- State storage combines SQLite and JSONL journal.
- Important domain concepts should retain Hanja labels in user-facing output and significant domain modeling.
- Product distribution should include a single binary, systemd user service support, and shell completion.

## Acceptance Criteria

- [ ] `cargo build` produces a usable `honbaek` binary.
- [ ] `honbaek awaken` initializes `~/.honbaek/`, starts or connects to the daemon, creates at least one `魂`, and records the event in SQLite and the JSONL journal.
- [ ] `honbaek assign "<task>"` submits a local repository task to an active `魂` and returns a task id.
- [ ] The daemon can execute a representative local repository task that reads files, writes or modifies a file, runs a verification command, records tool calls, and updates task status.
- [ ] `honbaek watch` opens a TUI-grade real-time view showing journal events, event timeline, current task, last action, provider usage, tool calls, and failure recovery state.
- [ ] `honbaek inspect` prints structured state for active `魂`, configured `魄`, current `心`, available `身`, durable `命`, active tasks, and recent journal entries.
- [ ] Multi-`魂` orchestration is usable from the CLI: multiple `魂` instances can be created or addressed, assigned independent tasks, and inspected separately.
- [ ] The OpenAI-compatible provider adapter can be configured through environment variables and non-secret config, and provider usage is visible in inspect/watch.
- [ ] `unbound` profiles are explicit, inspectable, and attached to `魂` execution state.
- [ ] `~/.honbaek/config.toml` never stores raw provider secrets.
- [ ] JSONL journal entries are append-only and contain enough information to reconstruct runtime history.
- [ ] SQLite state survives daemon restart.
- [ ] systemd user service files or install command support starting/stopping the daemon.
- [ ] shell completion can be generated or installed for the active shell.
- [ ] A usable smoke run demonstrates `awaken -> assign -> watch/inspect -> completed local repo artifact`.

## Implementation Boundaries

- Target area: `/home/argoss/dev/das1`.
- Expected modules:
  - CLI command parsing
  - daemon lifecycle and IPC
  - domain model for `魂`, `魄`, `心`, `身`, `命`
  - provider adapter boundary
  - OpenAI-compatible provider
  - task planner/executor loop
  - filesystem/shell/network tools
  - SQLite store
  - JSONL journal
  - TUI watch
  - inspect output
  - profile/permission model
  - systemd and shell completion support
- Preserve:
  - no hidden secret persistence
  - explicit unbound profile semantics
  - local-first product behavior
  - product-grade completion target

## Verification Plan

- `cargo fmt --check` should pass.
- `cargo clippy --workspace --all-targets -- -D warnings` should pass.
- `cargo test --workspace` should pass.
- `cargo build --release` should produce the binary.
- `honbaek awaken` should initialize local runtime state and record a journal event.
- `honbaek assign "create a short README status note for this workspace"` should produce a real local file artifact and a completed task record.
- `honbaek inspect` should show the active `魂`, configured `魄`, task state, provider/tool usage, and recent journal entries.
- `honbaek watch` should render a non-empty TUI view that reflects the current runtime state.
- Restarting the daemon should preserve SQLite state and journal history.
- systemd user service support and shell completion generation should be exercised far enough that the user can run the product locally.

## Open Questions

- None.

## Goal Handoff

Implement this spec exactly: `docs/specs/2026-05-08-honbaek-gangryeong.md`.
Treat the spec and active goal as the source of truth.
Keep scope limited to the acceptance criteria and implementation boundaries.
Delegate independent slices through Heddle when available.
Verify with the listed verification plan.
Mark complete only after the result is usable from the user's perspective.
