# 怪異 Extension Product Spec

## Source Request

`$interview-heddle 여기에 음.. 괴이를 넣어보고 싶어`

The user wants to add `怪異` to the existing 혼백강령 runtime without using Claude-backed workers. The extension must stay Codex-only while preserving the product-grade Rust CLI, daemon, TUI, SQLite, and JSONL runtime already built in `/home/argoss/dev/das1`.

## Goal

Add `怪異` as a first-class runtime concept and usable product feature. `怪異` represents anomalous entities that emerge from conflicts, failures, discontinuities, or unexplained runtime patterns among `魂`, `魄`, `心`, `身`, and `命`. The result must support automatic detection, manual recording, inspection, TUI visibility, local containment, and resolution tracking.

## Current Context

- Existing product spec: `docs/specs/2026-05-08-honbaek-gangryeong.md`.
- Existing Rust product: `honbaek`.
- Current CLI commands include `awaken`, `assign`, `watch`, `inspect`, `daemon`, `service`, and `completions`.
- Current domain concepts are modeled in `src/domain.rs` as `魂`, `魄`, `心`, `身`, and `命`.
- Current persistence uses SQLite and JSONL under `~/.honbaek/`.
- Current TUI `watch` and `inspect` already expose timeline, tasks, provider usage, tool calls, and failure recovery.

## Scope

- Add `怪異` as a new Hanja concept in the domain model and user-facing output.
- Add a persisted `怪異` anomaly entity with:
  - stable id
  - title
  - source kind
  - severity
  - lifecycle state
  - optional related task
  - evidence text
  - containment note
  - timestamps
- Add automatic detection for local runtime anomalies:
  - failed task execution
  - failed file read/write/verify tool operations
  - provider unavailable or provider/runtime fallback
  - daemon shutdown while work history exists
- Add manual recording through CLI.
- Add CLI operations under `honbaek kaeyi`:
  - `list`
  - `inspect <id>`
  - `scan`
  - `contain <id> --note <text>`
  - `resolve <id> --note <text>`
  - `record <title> --evidence <text> --severity <level>`
- Show `怪異` in `honbaek inspect`.
- Show `怪異` in both non-interactive and interactive `honbaek watch` TUI views.
- Keep all `怪異` side effects local: observe, warn, contain, and hold runtime state only. No file deletion, external network action, process kill, or destructive operation may run automatically.

## Non-Goals

- Do not turn `怪異` into a separate autonomous agent.
- Do not make `怪異` a decorative narrative-only layer.
- Do not implement a web UI.
- Do not add Claude-backed validation or Claude-backed workers.
- Do not perform destructive remediation automatically.
- Do not broaden provider support beyond what the existing provider adapter boundary already allows.

## Decisions

- `怪異` means an anomalous runtime entity created from conflict, failure, discontinuity, or unexplained patterns.
- Discovery is both automatic and manual.
- Runtime impact is warning, containment, and task-hold semantics only; destructive actions remain disallowed without explicit human approval.
- UI surface is TUI-first and all major local surfaces should expose it: `watch`, `inspect`, and dedicated CLI subcommands.
- Lifecycle states use Hanja labels:
  - `發現`: discovered
  - `觀測`: observed
  - `封印`: contained
  - `解消`: resolved
  - `歸屬`: attributed
- The implementation and review path must be Codex-only.

## Acceptance Criteria

- [ ] `怪異` exists as a first-class `Concept` and appears in event/journal output with the Hanja label.
- [ ] SQLite persists `怪異` records and survives daemon restart.
- [ ] JSONL journal records `怪異` discovery, containment, resolution, and manual record events.
- [ ] `honbaek kaeyi record <title> --evidence <text> --severity <level>` creates a manual `怪異`.
- [ ] `honbaek kaeyi list` prints persisted `怪異` entries with id, severity, state, title, and timestamp.
- [ ] `honbaek kaeyi inspect <id>` prints full detail for one `怪異`.
- [ ] `honbaek kaeyi scan` performs local anomaly detection and records newly discovered anomalies without destructive side effects.
- [ ] `honbaek kaeyi contain <id> --note <text>` changes state to `封印` and records the containment note.
- [ ] `honbaek kaeyi resolve <id> --note <text>` changes state to `解消` and records the resolution note.
- [ ] Failed task/tool/provider fallback paths automatically emit or upsert relevant `怪異` records.
- [ ] `honbaek inspect` shows current `怪異` summary and recent records.
- [ ] `honbaek watch --once` shows `怪異` summary, latest anomaly, and lifecycle state.
- [ ] Interactive `honbaek watch` shows `怪異` in the TUI without blocking the existing timeline/current/status panels.
- [ ] No automatic `怪異` path deletes files, kills processes, makes external network calls, or mutates external services.
- [ ] Existing 혼백강령 commands and smoke run continue to work.

## Implementation Boundaries

- Target area: `/home/argoss/dev/das1`.
- May touch:
  - `src/domain.rs`
  - `src/storage.rs`
  - `src/daemon.rs`
  - `src/executor.rs`
  - `src/ipc.rs`
  - `src/cli.rs`
  - `src/tui.rs`
  - `src/journal.rs`
  - `src/lib.rs`
  - `scripts/smoke.sh`
  - `README.md`
- Preserve:
  - existing command behavior
  - existing state location `~/.honbaek/`
  - no raw secret persistence
  - explicit `unbound` profile semantics
  - local-first operation
  - single Rust binary

## Verification Plan

- `cargo fmt --check` should pass.
- `cargo clippy --workspace --all-targets -- -D warnings` should pass.
- `cargo test --workspace` should pass.
- `cargo build --release` should pass.
- `target/release/honbaek kaeyi record "Manual omen" --evidence "operator observed unexpected runtime tension" --severity warning` should create a manual `怪異`.
- `target/release/honbaek kaeyi list` should show the manual `怪異`.
- `target/release/honbaek kaeyi inspect <id>` should show full detail and Hanja lifecycle state.
- `target/release/honbaek kaeyi contain <id> --note "held for observation"` should move the record to `封印`.
- `target/release/honbaek kaeyi resolve <id> --note "explained by manual audit"` should move the record to `解消`.
- `target/release/honbaek kaeyi scan` should complete without destructive side effects and record local anomalies when applicable.
- `target/release/honbaek inspect` should include a `怪異` section.
- `target/release/honbaek watch --once` should include `怪異` summary output.
- Interactive `target/release/honbaek watch` should render a non-empty TUI containing `怪異` information and exit with `q`.
- Existing `awaken -> assign -> inspect/watch` smoke path should still produce a completed local repo artifact.

## Open Questions

- None.

## Goal Handoff

Implement this spec exactly: `docs/specs/2026-05-08-kaeyi-extension.md`.
Treat the spec and active goal as the source of truth.
Keep scope limited to the acceptance criteria and implementation boundaries.
Use Heddle Codex workers only; do not use Claude-backed workers.
Verify with the listed verification plan.
Mark complete only after the result is usable from the user's perspective.
