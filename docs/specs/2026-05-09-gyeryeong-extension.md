# 戒令 Extension Product Spec

## Source Request

`$interview-heddle 계령 추가`

The user accepted the recommended interpretation: add `戒令` as a first-class runtime concept to Honbaek. `戒令` means runtime commandment, rule, prohibition, or standing order. It sits between `心` intent and `身` execution, checking work before local effects occur.

## Goal

Add `戒令` as a product-grade runtime feature. The result must let the operator define persistent local rules, inspect them, enable or disable them, and have Honbaek check enabled rules before assigned work reaches local execution. Rule violations or conflicts should be visible in the runtime and should create or update related `怪異` records.

## Current Context

- Existing product spec: `docs/specs/2026-05-08-honbaek-gangryeong.md`.
- Existing `怪異` extension spec: `docs/specs/2026-05-08-kaeyi-extension.md`.
- Existing Rust product: `honbaek`.
- Current runtime concepts are modeled in `src/domain.rs` as `魂`, `魄`, `心`, `身`, `命`, and `怪異`.
- Current persistence uses SQLite and JSONL under `~/.honbaek/`.
- Current CLI commands include `awaken`, `assign`, `watch`, `inspect`, `kaeyi`, `daemon`, `service`, and `completions`.
- Current `inspect` and `watch` output already expose `怪異` alongside core runtime state.
- Public README and banner expose the core Hanja concept set.

## Scope

- Add `戒令` as a new Hanja concept in the domain model and user-facing output.
- Add a persisted `戒令` entity with:
  - stable id
  - title
  - pattern text
  - action: `warn` or `block`
  - rationale text
  - enabled flag
  - timestamps
- Add CLI operations under `honbaek gyeryeong`:
  - `add <title> --pattern <text> --action <warn|block> --rationale <text>`
  - `list`
  - `inspect <id>`
  - `enable <id>`
  - `disable <id>`
- Check enabled `戒令` records before assigned task execution:
  - `warn` records a `戒令` event and allows execution to continue.
  - `block` records a `戒令` event, marks the task as failed with a clear blocked result, and prevents local executor side effects.
- Create or update a related `怪異` record when an enabled `戒令` matches a task prompt.
- Show `戒令` in `honbaek inspect`.
- Show `戒令` in `honbaek watch --once` and the interactive TUI.
- Update documentation and public presentation so the concept set includes `戒令`.

## Non-Goals

- Do not turn `戒令` into a separate autonomous agent.
- Do not add a web UI.
- Do not add remote policy management, accounts, or hosted sync.
- Do not execute destructive remediation automatically.
- Do not broaden provider support beyond the existing OpenAI-compatible boundary.
- Do not change the existing `怪異` lifecycle semantics except to link rule conflicts to anomaly records.
- Do not store provider secrets or sensitive data in `戒令` records.

## Decisions

- Hanja label is `戒令`.
- CLI command namespace is `gyeryeong`.
- `戒令` is a runtime rule layer between `心` and `身`.
- v1 scope is product-grade: CRUD-style management, enabled/disabled state, assign/executor preflight, inspect/watch visibility, and `怪異` linkage.
- Default rule action is `warn`; `block` must be explicit.
- A `戒令` conflict or violation automatically records a related `怪異`.
- The implementation remains local-first and single-binary.

## Acceptance Criteria

- [ ] `戒令` exists as a first-class `Concept` and appears in event/journal output with the Hanja label.
- [ ] SQLite persists `戒令` records and records survive daemon restart.
- [ ] JSONL journal records `戒令` add, enable, disable, warn, and block events.
- [ ] `honbaek gyeryeong add <title> --pattern <text> --action warn --rationale <text>` creates an enabled warning rule.
- [ ] `honbaek gyeryeong add <title> --pattern <text> --action block --rationale <text>` creates an enabled blocking rule.
- [ ] `honbaek gyeryeong list` prints persisted rules with id, action, enabled state, title, pattern, and timestamp.
- [ ] `honbaek gyeryeong inspect <id>` prints full detail for one rule.
- [ ] `honbaek gyeryeong disable <id>` prevents the rule from affecting future task assignments.
- [ ] `honbaek gyeryeong enable <id>` reactivates the rule for future task assignments.
- [ ] A matching enabled `warn` rule records a `戒令` event, links to `怪異`, and allows `honbaek assign` execution to continue.
- [ ] A matching enabled `block` rule records a `戒令` event, links to `怪異`, prevents executor side effects, and marks the task failed with a clear blocked result.
- [ ] Disabled rules do not warn, block, or create new `怪異` records.
- [ ] `honbaek inspect` shows current `戒令` summary and recent rules.
- [ ] `honbaek watch --once` shows `戒令` summary output.
- [ ] Interactive `honbaek watch` shows `戒令` without blocking the existing timeline/current/status/`怪異` panels.
- [ ] README, public banner, and package description include `戒令` in the concept set.
- [ ] Existing `怪異` commands and the smoke run continue to work.
- [ ] No automatic `戒令` path deletes files, kills processes, makes external network calls, or mutates external services.

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
  - `Cargo.toml`
  - `README.md`
  - `docs/assets/honbaek-banner.svg`
- Preserve:
  - existing command behavior
  - existing state location `~/.honbaek/`
  - no raw secret persistence
  - explicit `unbound` profile semantics
  - local-first operation
  - single Rust binary
  - existing `怪異` lifecycle labels and CLI behavior

## Verification Plan

- `cargo fmt --check` should pass.
- `cargo clippy --workspace --all-targets -- -D warnings` should pass.
- `cargo test --workspace` should pass.
- `cargo build --release` should pass.
- `target/release/honbaek gyeryeong add "No destructive prompt" --pattern "delete" --action warn --rationale "operator review required"` should create an enabled warning `戒令`.
- `target/release/honbaek gyeryeong list` should show the warning `戒令`.
- `target/release/honbaek gyeryeong inspect <id>` should show full detail.
- `target/release/honbaek assign "delete nothing; create a harmless status note"` should trigger the warning rule and still complete a local artifact.
- `target/release/honbaek gyeryeong add "Block forbidden prompt" --pattern "forbidden-gyeryeong-smoke" --action block --rationale "blocking path smoke"` should create an enabled blocking `戒令`.
- `target/release/honbaek assign "forbidden-gyeryeong-smoke"` should be blocked before executor side effects and should mark the task failed with a `戒令` result.
- `target/release/honbaek gyeryeong disable <id>` should disable the blocking rule.
- `target/release/honbaek assign "forbidden-gyeryeong-smoke"` should no longer be blocked by the disabled rule.
- `target/release/honbaek gyeryeong enable <id>` should re-enable the blocking rule.
- `target/release/honbaek kaeyi list` should include related `怪異` records for rule matches.
- `target/release/honbaek inspect` should include a `戒令` section.
- `target/release/honbaek watch --once` should include `戒令` summary output.
- Interactive `target/release/honbaek watch` should render a non-empty TUI containing `戒令` information and exit with `q`.
- `scripts/smoke.sh` should still pass and include a representative `戒令` path.

## Open Questions

- None.

## Goal Handoff

Implement this spec exactly: `docs/specs/2026-05-09-gyeryeong-extension.md`.
Treat the spec and active goal as the source of truth.
Keep scope limited to the acceptance criteria and implementation boundaries.
Delegate independent slices through Heddle when available.
Verify with the listed verification plan.
Mark complete only after the result is usable from the user's perspective.
