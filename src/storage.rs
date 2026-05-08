use crate::config::AppPaths;
use crate::domain::*;
use anyhow::{Result, bail};
use chrono::Utc;
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(paths: &AppPaths) -> Result<Self> {
        paths.ensure()?;
        Self::open_path(&paths.db)
    }

    pub fn open_path(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        configure_connection(&conn)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                name TEXT PRIMARY KEY,
                mode TEXT NOT NULL CHECK (mode IN ('safe', 'unbound')),
                permissions TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS hons (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                profile TEXT NOT NULL REFERENCES profiles(name) ON UPDATE CASCADE ON DELETE RESTRICT,
                active INTEGER NOT NULL CHECK (active IN (0, 1)),
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                hon_id TEXT NOT NULL REFERENCES hons(id) ON DELETE CASCADE,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed')),
                result TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                at TEXT NOT NULL,
                concept TEXT NOT NULL CHECK (concept IN ('魂', '魄', '心', '戒令', '身', '命', '怪異')),
                kind TEXT NOT NULL,
                message TEXT NOT NULL,
                task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS gyeryeong (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                pattern TEXT NOT NULL,
                action TEXT NOT NULL CHECK (action IN ('warn', 'block')),
                rationale TEXT NOT NULL,
                enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS kaeyi (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                source_kind TEXT NOT NULL,
                severity TEXT NOT NULL CHECK (severity IN ('low', 'warning', 'critical')),
                state TEXT NOT NULL CHECK (state IN ('發現', '觀測', '封印', '解消', '歸屬')),
                task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
                evidence TEXT NOT NULL,
                containment_note TEXT,
                resolution_note TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tool_calls (
                id TEXT PRIMARY KEY,
                task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
                tool TEXT NOT NULL,
                input TEXT NOT NULL,
                output TEXT NOT NULL,
                ok INTEGER NOT NULL CHECK (ok IN (0, 1)),
                at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS provider_usage (
                id TEXT PRIMARY KEY,
                task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                prompt_tokens INTEGER NOT NULL CHECK (prompt_tokens >= 0),
                completion_tokens INTEGER NOT NULL CHECK (completion_tokens >= 0),
                at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_hons_active_created ON hons(active, created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_updated ON tasks(updated_at);
            CREATE INDEX IF NOT EXISTS idx_events_at ON events(at);
            CREATE INDEX IF NOT EXISTS idx_tool_calls_at ON tool_calls(at);
            CREATE INDEX IF NOT EXISTS idx_provider_usage_at ON provider_usage(at);
            CREATE INDEX IF NOT EXISTS idx_gyeryeong_enabled ON gyeryeong(enabled, updated_at);
            CREATE INDEX IF NOT EXISTS idx_kaeyi_updated ON kaeyi(updated_at);
            CREATE INDEX IF NOT EXISTS idx_kaeyi_state ON kaeyi(state);
            "#,
        )?;
        self.migrate_events_for_current_concepts()?;
        self.seed_builtin_profiles()?;
        Ok(())
    }

    fn migrate_events_for_current_concepts(&self) -> Result<()> {
        let events_sql: Option<String> = self
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'events'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if events_sql
            .as_deref()
            .is_none_or(|sql| sql.contains("'怪異'") && sql.contains("'戒令'"))
        {
            return Ok(());
        }

        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = OFF;
            DROP INDEX IF EXISTS idx_events_at;
            ALTER TABLE events RENAME TO events_old;
            CREATE TABLE events (
                id TEXT PRIMARY KEY,
                at TEXT NOT NULL,
                concept TEXT NOT NULL CHECK (concept IN ('魂', '魄', '心', '戒令', '身', '命', '怪異')),
                kind TEXT NOT NULL,
                message TEXT NOT NULL,
                task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
                payload TEXT NOT NULL
            );
            INSERT INTO events (id, at, concept, kind, message, task_id, payload)
                SELECT id, at, concept, kind, message, task_id, payload FROM events_old;
            DROP TABLE events_old;
            CREATE INDEX IF NOT EXISTS idx_events_at ON events(at);
            PRAGMA foreign_keys = ON;
            "#,
        )?;
        Ok(())
    }

    pub fn ensure_profile(&self, name: &str, mode: AutonomyMode) -> Result<Profile> {
        let permissions = match mode {
            AutonomyMode::Safe => PermissionSet::safe(),
            AutonomyMode::Unbound => PermissionSet::unbound(),
        };
        let profile = Profile {
            name: name.to_string(),
            mode,
            permissions,
            created_at: Utc::now(),
        };
        self.conn.execute(
            "INSERT OR IGNORE INTO profiles (name, mode, permissions, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                profile.name,
                profile.mode.to_string(),
                serde_json::to_string(&profile.permissions)?,
                profile.created_at.to_rfc3339()
            ],
        )?;
        Ok(profile)
    }

    pub fn ensure_hon(&self, name: &str, profile: &str) -> Result<Hon> {
        if let Some(hon) = self.find_hon_by_name(name)? {
            return Ok(hon);
        }
        let profile_permissions = self.profile_permissions(profile)?;
        let hon = Hon {
            id: Uuid::new_v4(),
            name: name.to_string(),
            profile: profile.to_string(),
            profile_permissions,
            active: true,
            created_at: Utc::now(),
        };
        self.conn.execute(
            "INSERT INTO hons (id, name, profile, active, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                hon.id.to_string(),
                hon.name,
                hon.profile,
                hon.active as i64,
                hon.created_at.to_rfc3339()
            ],
        )?;
        Ok(hon)
    }

    pub fn find_hon_by_name(&self, name: &str) -> Result<Option<Hon>> {
        self.conn
            .query_row(
                "SELECT h.id, h.name, h.profile, p.permissions, h.active, h.created_at
                 FROM hons h
                 JOIN profiles p ON p.name = h.profile
                 WHERE h.name = ?1",
                params![name],
                row_to_hon,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn default_hon(&self) -> Result<Option<Hon>> {
        self.conn
            .query_row(
                "SELECT h.id, h.name, h.profile, p.permissions, h.active, h.created_at
                 FROM hons h
                 JOIN profiles p ON p.name = h.profile
                 WHERE h.active = 1
                 ORDER BY h.created_at
                 LIMIT 1",
                [],
                row_to_hon,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn insert_task(&self, hon_id: Uuid, prompt: &str) -> Result<Task> {
        let now = Utc::now();
        let task = Task {
            id: Uuid::new_v4(),
            hon_id,
            prompt: prompt.to_string(),
            status: TaskStatus::Queued,
            result: None,
            created_at: now,
            updated_at: now,
        };
        self.conn.execute(
            "INSERT INTO tasks (id, hon_id, prompt, status, result, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                task.id.to_string(),
                task.hon_id.to_string(),
                task.prompt,
                task.status.to_string(),
                task.result,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339()
            ],
        )?;
        Ok(task)
    }

    pub fn update_task(
        &self,
        task_id: Uuid,
        status: TaskStatus,
        result: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET status = ?1, result = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                status.to_string(),
                result,
                Utc::now().to_rfc3339(),
                task_id.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn insert_event(&self, event: &RuntimeEvent) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (id, at, concept, kind, message, task_id, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.id.to_string(),
                event.at.to_rfc3339(),
                event.concept.hanja(),
                event.kind,
                event.message,
                event.task_id.map(|id| id.to_string()),
                serde_json::to_string(&event.payload)?
            ],
        )?;
        Ok(())
    }

    pub fn insert_tool_call(&self, call: &ToolCall) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tool_calls (id, task_id, tool, input, output, ok, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                call.id.to_string(),
                call.task_id.map(|id| id.to_string()),
                call.tool,
                call.input,
                call.output,
                call.ok as i64,
                call.at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn insert_provider_usage(&self, usage: &ProviderUsage) -> Result<()> {
        self.conn.execute(
            "INSERT INTO provider_usage (id, task_id, provider, model, prompt_tokens, completion_tokens, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                usage.id.to_string(),
                usage.task_id.map(|id| id.to_string()),
                usage.provider,
                usage.model,
                usage.prompt_tokens as i64,
                usage.completion_tokens as i64,
                usage.at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn create_gyeryeong(
        &self,
        title: &str,
        pattern: &str,
        action: GyeryeongAction,
        rationale: &str,
    ) -> Result<Gyeryeong> {
        let title = title.trim();
        let pattern = pattern.trim();
        let rationale = rationale.trim();
        if title.is_empty() {
            bail!("戒令 title must not be empty");
        }
        if pattern.is_empty() {
            bail!("戒令 pattern must not be empty");
        }
        if rationale.is_empty() {
            bail!("戒令 rationale must not be empty");
        }

        let now = Utc::now();
        let gyeryeong = Gyeryeong {
            id: Uuid::new_v4(),
            title: title.to_string(),
            pattern: pattern.to_string(),
            action,
            rationale: rationale.to_string(),
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        self.conn.execute(
            "INSERT INTO gyeryeong (id, title, pattern, action, rationale, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                gyeryeong.id.to_string(),
                &gyeryeong.title,
                &gyeryeong.pattern,
                gyeryeong.action.to_string(),
                &gyeryeong.rationale,
                gyeryeong.enabled as i64,
                gyeryeong.created_at.to_rfc3339(),
                gyeryeong.updated_at.to_rfc3339()
            ],
        )?;
        self.get_gyeryeong(gyeryeong.id)?
            .ok_or_else(|| anyhow::anyhow!("created 戒令 disappeared before readback"))
    }

    pub fn list_gyeryeong(&self, limit: usize) -> Result<Vec<Gyeryeong>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, pattern, action, rationale, enabled, created_at, updated_at
             FROM gyeryeong
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_gyeryeong)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_enabled_gyeryeong(&self) -> Result<Vec<Gyeryeong>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, pattern, action, rationale, enabled, created_at, updated_at
             FROM gyeryeong
             WHERE enabled = 1
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_gyeryeong)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_gyeryeong(&self, id: Uuid) -> Result<Option<Gyeryeong>> {
        self.conn
            .query_row(
                "SELECT id, title, pattern, action, rationale, enabled, created_at, updated_at
                 FROM gyeryeong
                 WHERE id = ?1",
                params![id.to_string()],
                row_to_gyeryeong,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_gyeryeong_enabled(&self, id: Uuid, enabled: bool) -> Result<Gyeryeong> {
        self.conn.execute(
            "UPDATE gyeryeong SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled as i64, Utc::now().to_rfc3339(), id.to_string()],
        )?;
        self.get_gyeryeong(id)?
            .ok_or_else(|| anyhow::anyhow!("戒令 {id} does not exist"))
    }

    pub fn create_kaeyi(
        &self,
        title: &str,
        source_kind: &str,
        severity: KaeyiSeverity,
        task_id: Option<Uuid>,
        evidence: &str,
    ) -> Result<Kaeyi> {
        let now = Utc::now();
        let kaeyi = Kaeyi {
            id: Uuid::new_v4(),
            title: title.to_string(),
            source_kind: source_kind.to_string(),
            severity,
            state: KaeyiState::Discovered,
            task_id,
            evidence: evidence.to_string(),
            containment_note: None,
            resolution_note: None,
            created_at: now,
            updated_at: now,
        };
        self.conn.execute(
            "INSERT INTO kaeyi (id, title, source_kind, severity, state, task_id, evidence, containment_note, resolution_note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                kaeyi.id.to_string(),
                &kaeyi.title,
                &kaeyi.source_kind,
                kaeyi.severity.to_string(),
                kaeyi.state.to_string(),
                kaeyi.task_id.map(|id| id.to_string()),
                &kaeyi.evidence,
                &kaeyi.containment_note,
                &kaeyi.resolution_note,
                kaeyi.created_at.to_rfc3339(),
                kaeyi.updated_at.to_rfc3339()
            ],
        )?;
        self.get_kaeyi(kaeyi.id)?
            .ok_or_else(|| anyhow::anyhow!("created 怪異 disappeared before readback"))
    }

    pub fn upsert_kaeyi(
        &self,
        title: &str,
        source_kind: &str,
        severity: KaeyiSeverity,
        task_id: Option<Uuid>,
        evidence: &str,
    ) -> Result<KaeyiScanFinding> {
        if let Some(existing) = self.find_open_kaeyi(title, source_kind, task_id)? {
            self.conn.execute(
                "UPDATE kaeyi SET severity = ?1, evidence = ?2, updated_at = ?3 WHERE id = ?4",
                params![
                    severity.to_string(),
                    evidence,
                    Utc::now().to_rfc3339(),
                    existing.id.to_string()
                ],
            )?;
            let kaeyi = self
                .get_kaeyi(existing.id)?
                .ok_or_else(|| anyhow::anyhow!("updated 怪異 disappeared before readback"))?;
            return Ok(KaeyiScanFinding {
                kaeyi,
                created: false,
            });
        }

        Ok(KaeyiScanFinding {
            kaeyi: self.create_kaeyi(title, source_kind, severity, task_id, evidence)?,
            created: true,
        })
    }

    pub fn update_kaeyi_state(&self, id: Uuid, state: KaeyiState, note: &str) -> Result<Kaeyi> {
        let (containment_note, resolution_note) = match state {
            KaeyiState::Contained => (Some(note), None),
            KaeyiState::Resolved | KaeyiState::Attributed => (None, Some(note)),
            _ => (Some(note), None),
        };
        self.conn.execute(
            "UPDATE kaeyi
             SET state = ?1,
                 containment_note = COALESCE(?2, containment_note),
                 resolution_note = COALESCE(?3, resolution_note),
                 updated_at = ?4
             WHERE id = ?5",
            params![
                state.to_string(),
                containment_note,
                resolution_note,
                Utc::now().to_rfc3339(),
                id.to_string()
            ],
        )?;
        self.get_kaeyi(id)?
            .ok_or_else(|| anyhow::anyhow!("怪異 {id} does not exist"))
    }

    pub fn get_kaeyi(&self, id: Uuid) -> Result<Option<Kaeyi>> {
        self.conn
            .query_row(
                "SELECT id, title, source_kind, severity, state, task_id, evidence, containment_note, resolution_note, created_at, updated_at
                 FROM kaeyi
                 WHERE id = ?1",
                params![id.to_string()],
                row_to_kaeyi,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_kaeyi(&self, limit: usize) -> Result<Vec<Kaeyi>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, source_kind, severity, state, task_id, evidence, containment_note, resolution_note, created_at, updated_at
             FROM kaeyi
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_kaeyi)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn scan_kaeyi(&self) -> Result<Vec<KaeyiScanFinding>> {
        let mut findings = Vec::new();
        findings.extend(self.scan_failed_tasks()?);
        findings.extend(self.scan_failed_tools()?);
        findings.extend(self.scan_provider_fallbacks()?);
        if self.has_daemon_shutdown_event()? {
            findings.push(self.upsert_kaeyi(
                "Daemon shutdown with work history",
                "daemon.shutdown",
                KaeyiSeverity::Low,
                None,
                "journal contains daemon.shutdown while 命 work history exists",
            )?);
        }
        Ok(findings)
    }

    pub fn inspect_state(&self, baek: Baek) -> Result<InspectState> {
        let hons = self.list_hons()?;
        let profiles = self.list_profiles()?;
        let tasks = self.list_tasks(20)?;
        let events = self.list_events(30)?;
        let tool_calls = self.list_tool_calls(20)?;
        let provider_usage = self.list_provider_usage(20)?;
        let gyeryeong = self.list_gyeryeong(20)?;
        let kaeyi = self.list_kaeyi(20)?;
        let sim = Sim {
            current_intent: tasks.first().map(|task| task.prompt.clone()),
            priority: "autonomous local work".to_string(),
            self_check: "runtime state is inspectable".to_string(),
        };
        let shin = Shin {
            filesystem: true,
            shell: true,
            network: true,
        };
        let myeong = Myeong {
            identity: "혼백강령 local runtime".to_string(),
            continuity_events: events.len(),
        };
        Ok(InspectState {
            hons,
            profiles,
            baek,
            sim,
            gyeryeong,
            shin,
            myeong,
            kaeyi,
            tasks,
            events,
            tool_calls,
            provider_usage,
        })
    }

    fn list_hons(&self) -> Result<Vec<Hon>> {
        let mut stmt = self.conn.prepare(
            "SELECT h.id, h.name, h.profile, p.permissions, h.active, h.created_at
             FROM hons h
             JOIN profiles p ON p.name = h.profile
             ORDER BY h.created_at",
        )?;
        let rows = stmt.query_map([], row_to_hon)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn list_profiles(&self) -> Result<Vec<Profile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, mode, permissions, created_at FROM profiles ORDER BY name")?;
        let rows = stmt.query_map([], row_to_profile)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn list_tasks(&self, limit: usize) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hon_id, prompt, status, result, created_at, updated_at FROM tasks ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_task)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn list_events(&self, limit: usize) -> Result<Vec<RuntimeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, at, concept, kind, message, task_id, payload FROM events ORDER BY at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_event)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn list_tool_calls(&self, limit: usize) -> Result<Vec<ToolCall>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, tool, input, output, ok, at FROM tool_calls ORDER BY at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_tool_call)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn list_provider_usage(&self, limit: usize) -> Result<Vec<ProviderUsage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, provider, model, prompt_tokens, completion_tokens, at FROM provider_usage ORDER BY at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_provider_usage)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn profile_permissions(&self, profile: &str) -> Result<PermissionSet> {
        let permissions: Option<String> = self
            .conn
            .query_row(
                "SELECT permissions FROM profiles WHERE name = ?1",
                params![profile],
                |row| row.get(0),
            )
            .optional()?;
        match permissions {
            Some(raw) => Ok(serde_json::from_str(&raw)?),
            None => bail!("profile {profile} does not exist"),
        }
    }

    fn seed_builtin_profiles(&self) -> Result<()> {
        self.ensure_profile("safe", AutonomyMode::Safe)?;
        self.ensure_profile(DEFAULT_PROFILE_NAME, AutonomyMode::Unbound)?;
        Ok(())
    }

    fn find_open_kaeyi(
        &self,
        title: &str,
        source_kind: &str,
        task_id: Option<Uuid>,
    ) -> Result<Option<Kaeyi>> {
        self.conn
            .query_row(
                "SELECT id, title, source_kind, severity, state, task_id, evidence, containment_note, resolution_note, created_at, updated_at
                 FROM kaeyi
                 WHERE title = ?1
                   AND source_kind = ?2
                   AND ((task_id IS NULL AND ?3 IS NULL) OR task_id = ?3)
                   AND state NOT IN ('解消', '歸屬')
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![title, source_kind, task_id.map(|id| id.to_string())],
                row_to_kaeyi,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn has_work_history(&self) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    fn has_daemon_shutdown_event(&self) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM events WHERE kind = 'daemon.shutdown'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn scan_failed_tasks(&self) -> Result<Vec<KaeyiScanFinding>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hon_id, prompt, status, result, created_at, updated_at
             FROM tasks
             WHERE status = 'failed'
             ORDER BY updated_at DESC
             LIMIT 50",
        )?;
        let tasks = stmt
            .query_map([], row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut findings = Vec::new();
        for task in tasks {
            findings.push(self.upsert_kaeyi(
                "Task failure",
                "task.failed",
                KaeyiSeverity::Critical,
                Some(task.id),
                &format!(
                    "{}: {}",
                    task.prompt,
                    task.result.unwrap_or_else(|| "no result recorded".to_string())
                ),
            )?);
        }
        Ok(findings)
    }

    fn scan_failed_tools(&self) -> Result<Vec<KaeyiScanFinding>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, tool, input, output, ok, at
             FROM tool_calls
             WHERE ok = 0
             ORDER BY at DESC
             LIMIT 50",
        )?;
        let calls = stmt
            .query_map([], row_to_tool_call)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut findings = Vec::new();
        for call in calls {
            findings.push(self.upsert_kaeyi(
                &format!("Tool failure: {}", call.tool),
                "tool.failed",
                KaeyiSeverity::Warning,
                call.task_id,
                &format!("{} -> {}", call.input, call.output),
            )?);
        }
        Ok(findings)
    }

    fn scan_provider_fallbacks(&self) -> Result<Vec<KaeyiScanFinding>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, at, concept, kind, message, task_id, payload
             FROM events
             WHERE kind = 'provider.not_configured'
             ORDER BY at DESC
             LIMIT 50",
        )?;
        let events = stmt
            .query_map([], row_to_event)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut findings = Vec::new();
        for event in events {
            findings.push(self.upsert_kaeyi(
                "Provider fallback",
                "provider.not_configured",
                KaeyiSeverity::Warning,
                event.task_id,
                &event.message,
            )?);
        }
        Ok(findings)
    }
}

fn row_to_hon(row: &rusqlite::Row<'_>) -> rusqlite::Result<Hon> {
    let permissions: String = row.get(3)?;
    let created_at: String = row.get(5)?;
    Ok(Hon {
        id: parse_uuid(row.get::<_, String>(0)?),
        name: row.get(1)?,
        profile: row.get(2)?,
        profile_permissions: parse_permissions(permissions)?,
        active: row.get::<_, i64>(4)? != 0,
        created_at: parse_time(created_at),
    })
}

fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Profile> {
    let mode: String = row.get(1)?;
    let permissions: String = row.get(2)?;
    Ok(Profile {
        name: row.get(0)?,
        mode: parse_mode(&mode)?,
        permissions: parse_permissions(permissions)?,
        created_at: parse_time(row.get(3)?),
    })
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let status: String = row.get(3)?;
    Ok(Task {
        id: parse_uuid(row.get::<_, String>(0)?),
        hon_id: parse_uuid(row.get::<_, String>(1)?),
        prompt: row.get(2)?,
        status: parse_status(&status),
        result: row.get(4)?,
        created_at: parse_time(row.get(5)?),
        updated_at: parse_time(row.get(6)?),
    })
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<RuntimeEvent> {
    let concept: String = row.get(2)?;
    let task_id: Option<String> = row.get(5)?;
    let payload: String = row.get(6)?;
    Ok(RuntimeEvent {
        id: parse_uuid(row.get::<_, String>(0)?),
        at: parse_time(row.get(1)?),
        concept: parse_concept(&concept),
        kind: row.get(3)?,
        message: row.get(4)?,
        task_id: task_id.map(parse_uuid),
        payload: serde_json::from_str(&payload).unwrap_or_else(|_| serde_json::json!({})),
    })
}

fn row_to_tool_call(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolCall> {
    let task_id: Option<String> = row.get(1)?;
    Ok(ToolCall {
        id: parse_uuid(row.get::<_, String>(0)?),
        task_id: task_id.map(parse_uuid),
        tool: row.get(2)?,
        input: row.get(3)?,
        output: row.get(4)?,
        ok: row.get::<_, i64>(5)? != 0,
        at: parse_time(row.get(6)?),
    })
}

fn row_to_provider_usage(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderUsage> {
    let task_id: Option<String> = row.get(1)?;
    Ok(ProviderUsage {
        id: parse_uuid(row.get::<_, String>(0)?),
        task_id: task_id.map(parse_uuid),
        provider: row.get(2)?,
        model: row.get(3)?,
        prompt_tokens: row.get::<_, i64>(4)? as u64,
        completion_tokens: row.get::<_, i64>(5)? as u64,
        at: parse_time(row.get(6)?),
    })
}

fn row_to_gyeryeong(row: &rusqlite::Row<'_>) -> rusqlite::Result<Gyeryeong> {
    let action: String = row.get(3)?;
    Ok(Gyeryeong {
        id: parse_uuid(row.get::<_, String>(0)?),
        title: row.get(1)?,
        pattern: row.get(2)?,
        action: parse_gyeryeong_action(&action)?,
        rationale: row.get(4)?,
        enabled: row.get::<_, i64>(5)? != 0,
        created_at: parse_time(row.get(6)?),
        updated_at: parse_time(row.get(7)?),
    })
}

fn row_to_kaeyi(row: &rusqlite::Row<'_>) -> rusqlite::Result<Kaeyi> {
    let task_id: Option<String> = row.get(5)?;
    let severity: String = row.get(3)?;
    let state: String = row.get(4)?;
    Ok(Kaeyi {
        id: parse_uuid(row.get::<_, String>(0)?),
        title: row.get(1)?,
        source_kind: row.get(2)?,
        severity: parse_kaeyi_severity(&severity)?,
        state: parse_kaeyi_state(&state)?,
        task_id: task_id.map(parse_uuid),
        evidence: row.get(6)?,
        containment_note: row.get(7)?,
        resolution_note: row.get(8)?,
        created_at: parse_time(row.get(9)?),
        updated_at: parse_time(row.get(10)?),
    })
}

fn parse_uuid(value: String) -> Uuid {
    Uuid::parse_str(&value).unwrap_or_else(|_| Uuid::nil())
}

fn parse_time(value: String) -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|time| time.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_status(value: &str) -> TaskStatus {
    match value {
        "queued" => TaskStatus::Queued,
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Failed,
    }
}

fn parse_mode(value: &str) -> rusqlite::Result<AutonomyMode> {
    AutonomyMode::parse(value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            format!("invalid autonomy mode {value}").into(),
        )
    })
}

fn parse_permissions(value: String) -> rusqlite::Result<PermissionSet> {
    serde_json::from_str(&value)
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))
}

fn parse_concept(value: &str) -> Concept {
    match value {
        "魂" => Concept::Hon,
        "魄" => Concept::Baek,
        "心" => Concept::Sim,
        "戒令" => Concept::Gyeryeong,
        "身" => Concept::Shin,
        "命" => Concept::Myeong,
        "怪異" => Concept::Kaeyi,
        _ => Concept::Hon,
    }
}

fn parse_gyeryeong_action(value: &str) -> rusqlite::Result<GyeryeongAction> {
    value.parse().map_err(|error: String| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, error.into())
    })
}

fn parse_kaeyi_severity(value: &str) -> rusqlite::Result<KaeyiSeverity> {
    value.parse().map_err(|error: String| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, error.into())
    })
}

fn parse_kaeyi_state(value: &str) -> rusqlite::Result<KaeyiState> {
    KaeyiState::parse(value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            format!("invalid 怪異 state {value}").into(),
        )
    })
}

fn configure_connection(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn gyeryeong_records_persist_and_enabled_filter() {
        let path = temp_db_path();
        {
            let store = Store::open_path(&path).unwrap();
            let rule = store
                .create_gyeryeong(
                    "No destructive prompt",
                    "delete",
                    GyeryeongAction::Warn,
                    "operator review required",
                )
                .unwrap();
            assert!(rule.enabled);
            assert_eq!(store.list_enabled_gyeryeong().unwrap().len(), 1);

            let disabled = store.set_gyeryeong_enabled(rule.id, false).unwrap();
            assert!(!disabled.enabled);
            assert!(store.list_enabled_gyeryeong().unwrap().is_empty());
        }

        {
            let store = Store::open_path(&path).unwrap();
            let persisted = store.list_gyeryeong(10).unwrap();
            assert_eq!(persisted.len(), 1);
            assert_eq!(persisted[0].title, "No destructive prompt");
            assert_eq!(persisted[0].action, GyeryeongAction::Warn);
            assert!(!persisted[0].enabled);
        }

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("sqlite3-shm"));
        let _ = fs::remove_file(path.with_extension("sqlite3-wal"));
    }

    #[test]
    fn event_concept_parser_accepts_gyeryeong() {
        assert_eq!(parse_concept("戒令"), Concept::Gyeryeong);
    }

    fn temp_db_path() -> PathBuf {
        std::env::temp_dir().join(format!("honbaek-storage-test-{}.sqlite3", Uuid::new_v4()))
    }
}
