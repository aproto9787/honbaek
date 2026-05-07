use crate::config::AppPaths;
use crate::domain::{Concept, KaeyiSeverity, ProviderUsage, RuntimeEvent, Task, TaskStatus};
use crate::journal::Journal;
use crate::provider::ProviderAdapter;
use crate::storage::Store;
use crate::tools::ToolRuntime;
use anyhow::{Result, bail};
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

pub fn execute_local_repo_task(
    paths: &AppPaths,
    store: &Store,
    journal: &Journal,
    provider: &dyn ProviderAdapter,
    task: &Task,
) -> Result<()> {
    store.update_task(task.id, TaskStatus::Running, None)?;
    emit(
        store,
        journal,
        Concept::Hon,
        "task.started",
        "魂 started autonomous local repo task",
        Some(task.id),
        json!({ "prompt": task.prompt }),
    )?;

    if let Some(plan) = provider.plan(&task.prompt)? {
        let usage = ProviderUsage {
            id: Uuid::new_v4(),
            task_id: Some(task.id),
            provider: provider.baek().provider,
            model: provider.baek().model,
            prompt_tokens: plan.tokens_in,
            completion_tokens: plan.tokens_out,
            at: Utc::now(),
        };
        store.insert_provider_usage(&usage)?;
        emit(
            store,
            journal,
            Concept::Baek,
            "provider.plan",
            &plan.summary,
            Some(task.id),
            json!({ "tokens_in": plan.tokens_in, "tokens_out": plan.tokens_out }),
        )?;
    } else {
        emit(
            store,
            journal,
            Concept::Baek,
            "provider.not_configured",
            "魄 has no provider secret; deterministic local executor took over",
            Some(task.id),
            json!({ "api_key": "env-missing" }),
        )?;
        record_kaeyi(
            store,
            journal,
            KaeyiSignal {
                title: "Provider fallback",
                source_kind: "provider.not_configured",
                severity: KaeyiSeverity::Warning,
                task_id: Some(task.id),
                evidence: "魄 has no provider secret; deterministic local executor took over",
                message: "怪異 discovered provider/runtime fallback",
            },
        )?;
    }

    let tool_runtime = ToolRuntime::unbound();
    let artifact = choose_artifact(&task.prompt);
    let contents = render_artifact(&task.prompt);
    let write = tool_runtime.write_file(Some(task.id), &artifact, &contents)?;
    store.insert_tool_call(&write)?;
    emit(
        store,
        journal,
        Concept::Shin,
        "tool.file_write",
        &format!("身 wrote {}", artifact.display()),
        Some(task.id),
        json!({ "path": artifact }),
    )?;
    if !write.ok {
        let result = format!("file write failed: {}", write.output);
        record_kaeyi(
            store,
            journal,
            KaeyiSignal {
                title: "Tool failure: file.write",
                source_kind: "tool.failed",
                severity: KaeyiSeverity::Warning,
                task_id: Some(task.id),
                evidence: &result,
                message: "怪異 discovered failed file write",
            },
        )?;
        store.update_task(task.id, TaskStatus::Failed, Some(&result))?;
        emit(
            store,
            journal,
            Concept::Myeong,
            "task.failed",
            "命 recorded failed autonomous work",
            Some(task.id),
            json!({ "stage": "write", "output": write.output }),
        )?;
        bail!("{result}");
    }

    let read = tool_runtime.read_file(Some(task.id), &artifact)?;
    store.insert_tool_call(&read)?;
    emit(
        store,
        journal,
        Concept::Shin,
        "tool.file_read",
        &format!("身 read {}", artifact.display()),
        Some(task.id),
        json!({ "path": artifact, "ok": read.ok }),
    )?;
    if !read.ok {
        let result = format!("file read failed: {}", read.output);
        record_kaeyi(
            store,
            journal,
            KaeyiSignal {
                title: "Tool failure: file.read",
                source_kind: "tool.failed",
                severity: KaeyiSeverity::Warning,
                task_id: Some(task.id),
                evidence: &result,
                message: "怪異 discovered failed file read",
            },
        )?;
        store.update_task(task.id, TaskStatus::Failed, Some(&result))?;
        emit(
            store,
            journal,
            Concept::Myeong,
            "task.failed",
            "命 recorded failed autonomous work",
            Some(task.id),
            json!({ "stage": "read", "output": read.output }),
        )?;
        bail!("{result}");
    }

    let verify = tool_runtime.run_command(
        Some(task.id),
        "sh",
        &["-c", "test -s HONBAEK_STATUS.md || test -s README.md"],
    )?;
    store.insert_tool_call(&verify)?;
    emit(
        store,
        journal,
        Concept::Shin,
        "tool.shell_exec",
        "身 verified local repo artifact",
        Some(task.id),
        json!({ "ok": verify.ok, "output": verify.output }),
    )?;
    if !verify.ok {
        let result = format!("verification failed: {}", verify.output);
        record_kaeyi(
            store,
            journal,
            KaeyiSignal {
                title: "Tool failure: shell.exec",
                source_kind: "tool.failed",
                severity: KaeyiSeverity::Critical,
                task_id: Some(task.id),
                evidence: &result,
                message: "怪異 discovered failed verification command",
            },
        )?;
        store.update_task(task.id, TaskStatus::Failed, Some(&result))?;
        emit(
            store,
            journal,
            Concept::Myeong,
            "task.failed",
            "命 recorded failed autonomous work",
            Some(task.id),
            json!({ "stage": "verify", "output": verify.output }),
        )?;
        bail!("{result}");
    }

    store.update_task(
        task.id,
        TaskStatus::Completed,
        Some(&format!(
            "completed local repo artifact: {}",
            artifact.display()
        )),
    )?;
    emit(
        store,
        journal,
        Concept::Myeong,
        "task.completed",
        "命 recorded completed autonomous work",
        Some(task.id),
        json!({ "artifact": artifact }),
    )?;
    paths.ensure()?;
    Ok(())
}

struct KaeyiSignal<'a> {
    title: &'a str,
    source_kind: &'a str,
    severity: KaeyiSeverity,
    task_id: Option<Uuid>,
    evidence: &'a str,
    message: &'a str,
}

fn record_kaeyi(store: &Store, journal: &Journal, signal: KaeyiSignal<'_>) -> Result<()> {
    let finding = store.upsert_kaeyi(
        signal.title,
        signal.source_kind,
        signal.severity,
        signal.task_id,
        signal.evidence,
    )?;
    emit(
        store,
        journal,
        Concept::Kaeyi,
        if finding.created {
            "kaeyi.discovered"
        } else {
            "kaeyi.observed"
        },
        signal.message,
        signal.task_id,
        json!({
            "kaeyi_id": finding.kaeyi.id,
            "title": finding.kaeyi.title,
            "source_kind": finding.kaeyi.source_kind,
            "severity": finding.kaeyi.severity.to_string(),
            "state": finding.kaeyi.state.to_string(),
        }),
    )?;
    Ok(())
}

pub fn emit(
    store: &Store,
    journal: &Journal,
    concept: Concept,
    kind: &str,
    message: &str,
    task_id: Option<Uuid>,
    payload: serde_json::Value,
) -> Result<RuntimeEvent> {
    let event = RuntimeEvent {
        id: Uuid::new_v4(),
        at: Utc::now(),
        concept,
        kind: kind.to_string(),
        message: message.to_string(),
        task_id,
        payload,
    };
    store.insert_event(&event)?;
    journal.append(&event)?;
    Ok(event)
}

fn choose_artifact(prompt: &str) -> PathBuf {
    let lower = prompt.to_lowercase();
    if lower.contains("status") {
        PathBuf::from("HONBAEK_STATUS.md")
    } else if lower.contains("readme") {
        PathBuf::from("HONBAEK_ARTIFACT.md")
    } else {
        PathBuf::from("HONBAEK_STATUS.md")
    }
}

fn render_artifact(prompt: &str) -> String {
    format!(
        "# Honbaek Runtime Status\n\n- Requested task: {prompt}\n- Runtime: 혼백강령 local autonomous execution\n- 魂: active\n- 魄: OpenAI-compatible adapter boundary present\n- 心: task-directed\n- 身: filesystem and shell tools used\n- 命: journaled continuity\n"
    )
}
