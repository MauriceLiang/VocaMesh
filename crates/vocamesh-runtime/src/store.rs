use rusqlite::{params, Connection, OptionalExtension, Transaction};
use vocamesh_core::{
    ExecutionRequest, RecoveryStatus, RunEvent, RunRecord, RunState, ServiceError, TaskRecord,
};

const ERR_READ: ServiceError = ServiceError::new("storage.read-failed", "本地任务历史无法读取");
const ERR_WRITE: ServiceError = ServiceError::new("storage.write-failed", "本地任务历史无法保存");
const ERR_SCHEMA: ServiceError = ServiceError::new(
    "storage.schema-unsupported",
    "本地任务数据库版本不受支持，请保留数据库文件并联系维护者",
);
const EVENT_LIMIT_BYTES: usize = 2 * 1024 * 1024;
const EVENT_TEXT_LIMIT: usize = 64 * 1024;
const CATALOG_SCHEMA: &str = "
CREATE TABLE agent_roles (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, name_normalized TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL, provider_id TEXT NOT NULL CHECK (provider_id IN ('claude','codex','opencode')),
    description TEXT NOT NULL DEFAULT '', instructions TEXT NOT NULL DEFAULT '',
    archived_at INTEGER, revision INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE agent_permission_configs (
    agent_id TEXT PRIMARY KEY REFERENCES agent_roles(id),
    request_read INTEGER NOT NULL CHECK (request_read IN (0,1)),
    request_write INTEGER NOT NULL CHECK (request_write IN (0,1)),
    request_shell INTEGER NOT NULL CHECK (request_shell IN (0,1)),
    request_network INTEGER NOT NULL CHECK (request_network IN (0,1)),
    revision INTEGER NOT NULL DEFAULT 1, updated_at INTEGER NOT NULL
);
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, name_normalized TEXT NOT NULL UNIQUE,
    canonical_path TEXT NOT NULL UNIQUE, archived_at INTEGER,
    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE workspace_scopes (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('read','write','deny')),
    relative_path TEXT NOT NULL, PRIMARY KEY (workspace_id, scope_kind, relative_path)
);
CREATE TABLE teams (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, name_normalized TEXT NOT NULL UNIQUE,
    supervisor_agent_id TEXT NOT NULL REFERENCES agent_roles(id),
    workspace_id TEXT NOT NULL REFERENCES workspaces(id), archived_at INTEGER,
    revision INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE team_members (
    team_id TEXT NOT NULL REFERENCES teams(id), agent_id TEXT NOT NULL REFERENCES agent_roles(id),
    reports_to_agent_id TEXT REFERENCES agent_roles(id), PRIMARY KEY (team_id, agent_id)
);
CREATE INDEX idx_team_members_agent ON team_members(agent_id);
CREATE INDEX idx_teams_workspace ON teams(workspace_id);
CREATE TABLE catalog_audit (
    id TEXT PRIMARY KEY, object_type TEXT NOT NULL, object_id TEXT NOT NULL,
    action TEXT NOT NULL, revision INTEGER, happened_at INTEGER NOT NULL
);
CREATE TABLE catalog_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

pub(crate) struct Store(pub(crate) Connection);

pub(crate) struct RunFinish<'a> {
    pub(crate) run_state: RunState,
    pub(crate) outcome: &'a str,
    pub(crate) task_state: &'a str,
    pub(crate) exit_code: Option<i32>,
    pub(crate) summary: Option<&'a str>,
    pub(crate) error_code: Option<&'a str>,
    pub(crate) pid: Option<u32>,
}

fn migrate_catalog_schema(connection: &Connection) -> Result<(), ServiceError> {
    let migration = format!("BEGIN IMMEDIATE; {CATALOG_SCHEMA} PRAGMA user_version = 3; COMMIT;");
    connection.execute_batch(&migration).map_err(|_| ERR_WRITE)
}

impl Store {
    pub(crate) fn open(path: &std::path::Path) -> Result<Self, ServiceError> {
        let connection = Connection::open(path).map_err(|_| ERR_READ)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|_| ERR_READ)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| ERR_READ)?;
        if version > 3 {
            return Err(ERR_SCHEMA);
        }
        if version == 0 {
            let schema = format!(
                "BEGIN IMMEDIATE;
                 CREATE TABLE tasks (
                    id TEXT PRIMARY KEY, title TEXT NOT NULL, prompt TEXT NOT NULL,
                    provider_id TEXT NOT NULL, workspace_path TEXT NOT NULL, state TEXT NOT NULL,
                    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
                    current_run_id TEXT NOT NULL, consent_at INTEGER NOT NULL,
                    authorization_scope TEXT NOT NULL
                 );
                 CREATE TABLE runs (
                    id TEXT PRIMARY KEY, request_id TEXT NOT NULL UNIQUE,
                    task_id TEXT NOT NULL REFERENCES tasks(id), provider_id TEXT NOT NULL,
                    state TEXT NOT NULL, outcome TEXT, pid INTEGER,
                    process_group_id INTEGER,
                    recovery_status TEXT NOT NULL DEFAULT 'none'
                        CHECK (recovery_status IN ('none','required','resolved','manual_required')),
                    recovery_checked_at INTEGER,
                    started_at INTEGER, finished_at INTEGER, exit_code INTEGER,
                    error_code TEXT, summary TEXT, last_sequence INTEGER NOT NULL DEFAULT 0,
                    output_bytes INTEGER NOT NULL DEFAULT 0, output_truncated INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE INDEX runs_task_id ON runs(task_id, started_at DESC);
                 CREATE INDEX runs_state ON runs(state);
                 CREATE UNIQUE INDEX one_active_run ON runs((1)) WHERE state IN ('pending','starting','running','stop-requested');
                 CREATE TABLE run_events (
                    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
                    sequence INTEGER NOT NULL, timestamp INTEGER NOT NULL,
                    kind TEXT NOT NULL, text TEXT NOT NULL,
                    PRIMARY KEY(run_id, sequence)
                 );
                 {CATALOG_SCHEMA}
                 PRAGMA user_version = 3;
                 COMMIT;"
            );
            connection.execute_batch(&schema).map_err(|_| ERR_WRITE)?;
        } else if version == 1 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                 ALTER TABLE runs ADD COLUMN process_group_id INTEGER;
                 ALTER TABLE runs ADD COLUMN recovery_status TEXT NOT NULL DEFAULT 'none'
                    CHECK (recovery_status IN ('none','required','resolved','manual_required'));
                 ALTER TABLE runs ADD COLUMN recovery_checked_at INTEGER;
                 PRAGMA user_version = 2;
                 COMMIT;",
                )
                .map_err(|_| ERR_WRITE)?;
            migrate_catalog_schema(&connection)?;
        } else if version == 2 {
            migrate_catalog_schema(&connection)?;
        }
        Ok(Self(connection))
    }

    pub(crate) fn recover_active(&mut self) -> Result<(), ServiceError> {
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        let timestamp = now();
        let mut statement = tx
            .prepare("SELECT id, process_group_id FROM runs WHERE state IN ('pending','starting','running','stop-requested')")
            .map_err(|_| ERR_READ)?;
        let active = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
            })
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        drop(statement);
        for (run_id, process_group_id) in active {
            let recovery_status = if process_group_id.is_some() {
                "required"
            } else {
                "manual_required"
            };
            tx.execute(
                "UPDATE runs SET state='unknown', outcome='unknown', finished_at=?2,
                    error_code=COALESCE(error_code,'run.recovered-unknown'), recovery_status=?3
                 WHERE id=?1",
                params![run_id, timestamp, recovery_status],
            )
            .map_err(|_| ERR_WRITE)?;
            let task_id: String = tx
                .query_row("SELECT task_id FROM runs WHERE id=?1", [&run_id], |row| {
                    row.get(0)
                })
                .map_err(|_| ERR_READ)?;
            tx.execute(
                "UPDATE tasks SET state='unknown', updated_at=?2 WHERE id=?1",
                params![task_id, timestamp],
            )
            .map_err(|_| ERR_WRITE)?;
            insert_audit_event(
                &tx,
                &run_id,
                "recovery-required",
                "进程状态未确认；新任务已暂停",
            )?;
        }

        let mut statement = tx
            .prepare("SELECT id, process_group_id FROM runs WHERE state='unknown' AND recovery_status='none'")
            .map_err(|_| ERR_READ)?;
        let legacy_unknown = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
            })
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        drop(statement);
        for (run_id, process_group_id) in legacy_unknown {
            let recovery_status = if process_group_id.is_some() {
                "required"
            } else {
                "manual_required"
            };
            tx.execute(
                "UPDATE runs SET recovery_status=?2, error_code=COALESCE(error_code,'run.recovered-unknown') WHERE id=?1",
                params![run_id, recovery_status],
            )
            .map_err(|_| ERR_WRITE)?;
            insert_audit_event(
                &tx,
                &run_id,
                "recovery-required",
                "历史运行状态未确认；新任务已暂停",
            )?;
        }
        tx.commit().map_err(|_| ERR_WRITE)
    }

    pub(crate) fn request_exists(&self, request_id: &str) -> Result<bool, ServiceError> {
        self.0
            .query_row(
                "SELECT 1 FROM runs WHERE request_id=?1",
                [request_id],
                |_| Ok(()),
            )
            .optional()
            .map(|x| x.is_some())
            .map_err(|_| ERR_READ)
    }

    pub(crate) fn has_active(&self) -> Result<bool, ServiceError> {
        self.0.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE state IN ('pending','starting','running','stop-requested'))", [], |r| r.get(0)).map_err(|_| ERR_READ)
    }

    pub(crate) fn has_uncertain_process(&self) -> Result<bool, ServiceError> {
        self.0
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM runs WHERE state='unknown')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| ERR_READ)
    }

    pub(crate) fn create_pending(
        &mut self,
        request: &ExecutionRequest,
        workspace: &str,
    ) -> Result<(TaskRecord, RunRecord), ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let uncertain: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM runs WHERE state='unknown')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| ERR_READ)?;
        if uncertain {
            return Err(ServiceError::new(
                "execution.process-state-unknown",
                "先前 Claude Code 进程状态无法确认；为避免重复启动，真实执行已暂停",
            ));
        }
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM runs WHERE state IN ('pending','starting','running','stop-requested'))",
                [],
                |row| row.get(0),
            )
            .map_err(|_| ERR_READ)?;
        if active {
            return Err(ServiceError::new(
                "execution.already-running",
                "当前已有一个本机 Run 正在执行",
            ));
        }
        let timestamp = now();
        let task_id = new_id(&tx, "task")?;
        let run_id = new_id(&tx, "run")?;
        tx.execute("INSERT INTO tasks(id,title,prompt,provider_id,workspace_path,state,created_at,updated_at,current_run_id,consent_at,authorization_scope) VALUES(?1,?2,?3,?4,?5,'queued',?6,?6,?7,?6,'read-only-filtered-snapshot')", params![task_id, request.title, request.prompt, request.provider_id, workspace, timestamp, run_id]).map_err(|_| ERR_WRITE)?;
        tx.execute("INSERT INTO runs(id,request_id,task_id,provider_id,state) VALUES(?1,?2,?3,?4,'pending')", params![run_id, request.request_id, task_id, request.provider_id]).map_err(|_| ERR_WRITE)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        Ok((self.get_task(&task_id)?, self.get_run(&run_id)?))
    }

    pub(crate) fn get_task(&self, id: &str) -> Result<TaskRecord, ServiceError> {
        self.0.query_row("SELECT id,title,prompt,provider_id,workspace_path,state,created_at,updated_at,current_run_id,authorization_scope,consent_at FROM tasks WHERE id=?1", [id], map_task).optional().map_err(|_| ERR_READ)?.ok_or_else(|| ServiceError::new("task.not-found", "找不到该任务"))
    }

    pub(crate) fn get_run(&self, id: &str) -> Result<RunRecord, ServiceError> {
        self.0.query_row("SELECT id,request_id,task_id,provider_id,state,outcome,started_at,finished_at,exit_code,summary,last_sequence,output_truncated,pid,error_code,recovery_status,recovery_checked_at FROM runs WHERE id=?1", [id], map_run).optional().map_err(|_| ERR_READ)?.ok_or_else(|| ServiceError::new("run.not-found", "找不到该 Run"))
    }

    pub(crate) fn process_group_id(&self, id: &str) -> Result<Option<i64>, ServiceError> {
        self.0
            .query_row(
                "SELECT process_group_id FROM runs WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ERR_READ)?
            .ok_or_else(|| ServiceError::new("run.not-found", "找不到该 Run"))
    }

    pub(crate) fn list_unknown_runs(&self) -> Result<Vec<(RunRecord, String)>, ServiceError> {
        let mut statement = self
            .0
            .prepare(
                "SELECT id,request_id,task_id,provider_id,state,outcome,started_at,finished_at,
                exit_code,summary,last_sequence,output_truncated,pid,error_code,recovery_status,
                recovery_checked_at, tasks.title
             FROM runs JOIN tasks ON tasks.id=runs.task_id WHERE runs.state='unknown'
             ORDER BY runs.finished_at DESC, runs.started_at DESC",
            )
            .map_err(|_| ERR_READ)?;
        let rows = statement
            .query_map([], |row| Ok((map_run(row)?, row.get::<_, String>(16)?)))
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        Ok(rows)
    }

    pub(crate) fn inspect_unknown_run(&mut self, run_id: &str) -> Result<(), ServiceError> {
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        let changed = tx
            .execute(
                "UPDATE runs SET recovery_checked_at=?2
             WHERE id=?1 AND state='unknown'",
                params![run_id, now()],
            )
            .map_err(|_| ERR_WRITE)?;
        if changed == 0 {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1)",
                    [run_id],
                    |row| row.get(0),
                )
                .map_err(|_| ERR_READ)?;
            if !exists {
                return Err(ServiceError::new("run.not-found", "找不到该 Run"));
            }
            return Err(ServiceError::new(
                "run.recovery-not-required",
                "该 Run 当前不需要恢复检查",
            ));
        }
        tx.commit().map_err(|_| ERR_WRITE)
    }

    pub(crate) fn resolve_unknown_run(&mut self, run_id: &str) -> Result<RunRecord, ServiceError> {
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        let timestamp = now();
        let changed = tx.execute(
            "UPDATE runs SET state='crashed', outcome='recovered-after-exit', finished_at=?2,
                recovery_status='resolved', recovery_checked_at=?2, pid=NULL, process_group_id=NULL
             WHERE id=?1 AND state='unknown' AND recovery_status='required' AND process_group_id IS NOT NULL",
            params![run_id, timestamp],
        ).map_err(|_| ERR_WRITE)?;
        if changed == 0 {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1)",
                    [run_id],
                    |row| row.get(0),
                )
                .map_err(|_| ERR_READ)?;
            if !exists {
                return Err(ServiceError::new("run.not-found", "找不到该 Run"));
            }
            return Err(ServiceError::new(
                "run.recovery-not-required",
                "该 Run 当前不满足归档条件",
            ));
        }
        tx.execute(
            "UPDATE tasks SET state='failed', updated_at=?2 WHERE current_run_id=?1",
            params![run_id, timestamp],
        )
        .map_err(|_| ERR_WRITE)?;
        insert_audit_event(
            &tx,
            run_id,
            "recovery-confirmed-exited",
            "用户已人工核验相关进程并确认归档；任务未重跑",
        )?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        self.get_run(run_id)
    }

    pub(crate) fn record_cleanup_failure(&mut self, run_id: &str) -> Result<(), ServiceError> {
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        insert_audit_event(
            &tx,
            run_id,
            "recovery-snapshot-cleanup-failed",
            "恢复已归档，但快照清理失败；请检查本地存储",
        )?;
        tx.commit().map_err(|_| ERR_WRITE)
    }

    pub(crate) fn by_request(
        &self,
        request_id: &str,
    ) -> Result<(TaskRecord, RunRecord), ServiceError> {
        let run_id: String = self
            .0
            .query_row(
                "SELECT id FROM runs WHERE request_id=?1",
                [request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| ERR_READ)?
            .ok_or(ERR_READ)?;
        let run = self.get_run(&run_id)?;
        Ok((self.get_task(&run.task_id)?, run))
    }

    pub(crate) fn list_tasks(&self) -> Result<Vec<TaskRecord>, ServiceError> {
        let mut statement = self.0.prepare("SELECT id,title,prompt,provider_id,workspace_path,state,created_at,updated_at,current_run_id,authorization_scope,consent_at FROM tasks ORDER BY created_at DESC LIMIT 500").map_err(|_| ERR_READ)?;
        let rows = statement
            .query_map([], map_task)
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ);
        rows
    }

    pub(crate) fn list_runs(
        &self,
        cursor: Option<u64>,
        limit: usize,
    ) -> Result<(Vec<RunRecord>, bool, Option<u64>), ServiceError> {
        let mut statement = self.0.prepare("SELECT id,request_id,task_id,provider_id,state,outcome,started_at,finished_at,exit_code,summary,last_sequence,output_truncated,pid,error_code,recovery_status,recovery_checked_at FROM runs WHERE (?1 IS NULL OR rowid < ?1) ORDER BY rowid DESC LIMIT ?2").map_err(|_| ERR_READ)?;
        let limit = limit.clamp(1, 100);
        let mut items = statement
            .query_map(
                params![cursor.map(|c| c as i64), (limit + 1) as i64],
                map_run,
            )
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = if has_more {
            items
                .last()
                .map(|run| {
                    self.0
                        .query_row("SELECT rowid FROM runs WHERE id=?1", [&run.id], |row| {
                            row.get::<_, i64>(0)
                        })
                        .map(|rowid| rowid as u64)
                })
                .transpose()
                .map_err(|_| ERR_READ)?
        } else {
            None
        };
        Ok((items, has_more, next_cursor))
    }

    pub(crate) fn list_events(
        &self,
        run_id: &str,
        after: u64,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ServiceError> {
        let mut statement = self.0.prepare("SELECT run_id,sequence,timestamp,kind,text FROM run_events WHERE run_id=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3").map_err(|_| ERR_READ)?;
        let rows = statement
            .query_map(
                params![run_id, after as i64, limit.clamp(1, 100) as i64],
                |row| {
                    Ok(RunEvent {
                        run_id: row.get(0)?,
                        sequence: row.get::<_, i64>(1)? as u64,
                        timestamp: row.get::<_, i64>(2)? as u64,
                        kind: row.get(3)?,
                        text: row.get(4)?,
                    })
                },
            )
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ);
        rows
    }

    pub(crate) fn set_state(
        &mut self,
        run_id: &str,
        state: RunState,
        task_state: &str,
        pid: Option<u32>,
        process_group_id: Option<u32>,
    ) -> Result<RunRecord, ServiceError> {
        let current = self.get_run(run_id)?;
        let transition_allowed = match state {
            RunState::Starting => current.state == RunState::Pending,
            RunState::Running => current.state == RunState::Starting,
            RunState::StopRequested => current.state.is_active(),
            _ => false,
        };
        if !transition_allowed {
            return Ok(current);
        }
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        let timestamp = now();
        let changed = tx.execute("UPDATE runs SET state=?2, started_at=COALESCE(started_at, CASE WHEN ?2='running' THEN ?3 END), pid=COALESCE(?4,pid), process_group_id=COALESCE(?5,process_group_id) WHERE id=?1 AND state IN ('pending','starting','running','stop-requested')", params![run_id, run_state_as_db(state), timestamp, pid.map(i64::from), process_group_id.map(i64::from)]).map_err(|_| ERR_WRITE)?;
        if changed == 0 {
            tx.commit().map_err(|_| ERR_WRITE)?;
            return self.get_run(run_id);
        }
        tx.execute(
            "UPDATE tasks SET state=?2, updated_at=?3 WHERE current_run_id=?1",
            params![run_id, task_state, timestamp],
        )
        .map_err(|_| ERR_WRITE)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        self.get_run(run_id)
    }

    pub(crate) fn append_event(
        &mut self,
        run_id: &str,
        kind: &str,
        text: &str,
    ) -> Result<Option<RunEvent>, ServiceError> {
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        let (sequence, bytes, truncated): (i64, i64, bool) = tx
            .query_row(
                "SELECT last_sequence,output_bytes,output_truncated FROM runs WHERE id=?1",
                [run_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|_| ERR_READ)?
            .ok_or_else(|| ServiceError::new("run.not-found", "找不到该 Run"))?;
        if truncated {
            return Ok(None);
        }
        let mut end = text.len().min(EVENT_TEXT_LIMIT);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let content = &text[..end];
        let next_bytes = content.len();
        if bytes as usize + next_bytes > EVENT_LIMIT_BYTES {
            if truncated {
                return Ok(None);
            }
            let next = sequence + 1;
            tx.execute("INSERT INTO run_events(run_id,sequence,timestamp,kind,text) VALUES(?1,?2,?3,'truncated','输出已达到本 Run 的 2 MiB 上限，后续内容不再保存')", params![run_id,next,now()]).map_err(|_| ERR_WRITE)?;
            tx.execute(
                "UPDATE runs SET last_sequence=?2,output_truncated=1 WHERE id=?1",
                params![run_id, next],
            )
            .map_err(|_| ERR_WRITE)?;
            tx.commit().map_err(|_| ERR_WRITE)?;
            return Ok(Some(RunEvent {
                run_id: run_id.into(),
                sequence: next as u64,
                timestamp: now() as u64,
                kind: "truncated".into(),
                text: "输出已达到本 Run 的 2 MiB 上限，后续内容不再保存".into(),
            }));
        }
        let next = sequence + 1;
        let timestamp = now();
        tx.execute(
            "INSERT INTO run_events(run_id,sequence,timestamp,kind,text) VALUES(?1,?2,?3,?4,?5)",
            params![run_id, next, timestamp, kind, content],
        )
        .map_err(|_| ERR_WRITE)?;
        tx.execute(
            "UPDATE runs SET last_sequence=?2,output_bytes=output_bytes+?3 WHERE id=?1",
            params![run_id, next, next_bytes as i64],
        )
        .map_err(|_| ERR_WRITE)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        Ok(Some(RunEvent {
            run_id: run_id.into(),
            sequence: next as u64,
            timestamp: timestamp as u64,
            kind: kind.into(),
            text: content.to_owned(),
        }))
    }

    pub(crate) fn finish(
        &mut self,
        run_id: &str,
        finish: RunFinish<'_>,
    ) -> Result<RunRecord, ServiceError> {
        let tx = self.0.transaction().map_err(|_| ERR_WRITE)?;
        let timestamp = now();
        let changed = tx.execute("UPDATE runs SET state=?2,outcome=?3,finished_at=?4,exit_code=?5,summary=?6,error_code=CASE WHEN ?2='unknown' THEN COALESCE(error_code,?7) ELSE ?7 END,pid=CASE WHEN ?2='unknown' THEN COALESCE(pid,?8) ELSE NULL END,process_group_id=CASE WHEN ?2='unknown' THEN process_group_id ELSE NULL END,recovery_status=CASE WHEN ?2='unknown' THEN CASE WHEN process_group_id IS NULL THEN 'manual_required' ELSE 'required' END ELSE recovery_status END WHERE id=?1 AND state IN ('pending','starting','running','stop-requested')", params![run_id,run_state_as_db(finish.run_state),finish.outcome,timestamp,finish.exit_code,finish.summary,finish.error_code,finish.pid.map(i64::from)]).map_err(|_| ERR_WRITE)?;
        if changed == 0 {
            tx.commit().map_err(|_| ERR_WRITE)?;
            return self.get_run(run_id);
        }
        tx.execute(
            "UPDATE tasks SET state=?2,updated_at=?3 WHERE current_run_id=?1",
            params![run_id, finish.task_state, timestamp],
        )
        .map_err(|_| ERR_WRITE)?;
        if finish.run_state == RunState::Unknown {
            insert_audit_event(
                &tx,
                run_id,
                "recovery-required",
                "进程状态未确认；新任务已暂停",
            )?;
        }
        tx.commit().map_err(|_| ERR_WRITE)?;
        self.get_run(run_id)
    }

    pub(crate) fn has_request(
        &self,
        request_id: &str,
    ) -> Result<Option<(TaskRecord, RunRecord)>, ServiceError> {
        if self.request_exists(request_id)? {
            self.by_request(request_id).map(Some)
        } else {
            Ok(None)
        }
    }
}

fn run_state_as_db(state: RunState) -> &'static str {
    match state {
        RunState::Pending => "pending",
        RunState::Starting => "starting",
        RunState::Running => "running",
        RunState::StopRequested => "stop-requested",
        RunState::Finished => "finished",
        RunState::Crashed => "crashed",
        RunState::Unknown => "unknown",
    }
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        prompt: row.get(2)?,
        provider_id: row.get(3)?,
        workspace_path: row.get(4)?,
        state: row.get(5)?,
        created_at: row.get::<_, i64>(6)? as u64,
        updated_at: row.get::<_, i64>(7)? as u64,
        current_run_id: row.get(8)?,
        authorization_scope: row.get(9)?,
        consent_at: row.get::<_, i64>(10)? as u64,
    })
}

fn map_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRecord> {
    let state: String = row.get(4)?;
    Ok(RunRecord {
        id: row.get(0)?,
        request_id: row.get(1)?,
        task_id: row.get(2)?,
        provider_id: row.get(3)?,
        state: match state.as_str() {
            "pending" => RunState::Pending,
            "starting" => RunState::Starting,
            "running" => RunState::Running,
            "stop-requested" => RunState::StopRequested,
            "finished" => RunState::Finished,
            "crashed" => RunState::Crashed,
            _ => RunState::Unknown,
        },
        outcome: row.get(5)?,
        started_at: row.get::<_, Option<i64>>(6)?.map(|x| x as u64),
        finished_at: row.get::<_, Option<i64>>(7)?.map(|x| x as u64),
        exit_code: row.get(8)?,
        summary: row.get(9)?,
        last_sequence: row.get::<_, i64>(10)? as u64,
        output_truncated: row.get(11)?,
        process_id: row.get::<_, Option<i64>>(12)?.map(|pid| pid as u32),
        error_code: row.get(13)?,
        recovery_status: match row.get::<_, String>(14)?.as_str() {
            "required" => RecoveryStatus::Required,
            "resolved" => RecoveryStatus::Resolved,
            "manual_required" => RecoveryStatus::ManualRequired,
            _ => RecoveryStatus::None,
        },
        recovery_checked_at: row.get::<_, Option<i64>>(15)?.map(|value| value as u64),
    })
}

fn insert_audit_event(
    tx: &Transaction<'_>,
    run_id: &str,
    kind: &str,
    text: &str,
) -> Result<(), ServiceError> {
    let sequence: i64 = tx
        .query_row(
            "SELECT last_sequence FROM runs WHERE id=?1",
            [run_id],
            |row| row.get(0),
        )
        .map_err(|_| ERR_READ)?;
    let next = sequence + 1;
    tx.execute(
        "INSERT INTO run_events(run_id,sequence,timestamp,kind,text) VALUES(?1,?2,?3,?4,?5)",
        params![run_id, next, now(), kind, text],
    )
    .map_err(|_| ERR_WRITE)?;
    tx.execute(
        "UPDATE runs SET last_sequence=?2 WHERE id=?1",
        params![run_id, next],
    )
    .map_err(|_| ERR_WRITE)?;
    Ok(())
}

pub(crate) fn new_id(tx: &Transaction<'_>, prefix: &str) -> Result<String, ServiceError> {
    let random: String = tx
        .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
        .map_err(|_| ERR_WRITE)?;
    Ok(format!("{prefix}_{random}"))
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn request(id: &str) -> ExecutionRequest {
        ExecutionRequest {
            request_id: id.into(),
            title: "migration history".into(),
            prompt: "synthetic prompt".into(),
            provider_id: "claude".into(),
            workspace_path: "/tmp/synthetic-workspace".into(),
            consent: true,
        }
    }

    fn downgrade_to_v1(path: &std::path::Path) {
        Connection::open(path)
            .unwrap()
            .execute_batch(
                "DROP TABLE team_members;
                 DROP TABLE teams;
                 DROP TABLE workspace_scopes;
                 DROP TABLE workspaces;
                 DROP TABLE agent_permission_configs;
                 DROP TABLE agent_roles;
                 DROP TABLE catalog_audit;
                 DROP TABLE catalog_settings;
                 ALTER TABLE runs DROP COLUMN process_group_id;
                 ALTER TABLE runs DROP COLUMN recovery_status;
                 ALTER TABLE runs DROP COLUMN recovery_checked_at;
                 PRAGMA user_version = 1;",
            )
            .unwrap();
    }

    #[test]
    fn empty_database_initializes_schema_v3_without_catalog_seeds() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        let _store = Store::open(&path).unwrap();
        let connection = Connection::open(path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
        let columns = connection
            .prepare("PRAGMA table_info(runs)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.contains(&"process_group_id".into()));
        assert!(columns.contains(&"recovery_status".into()));
        assert!(columns.contains(&"recovery_checked_at".into()));
        let catalog_agents: i64 = connection
            .query_row("SELECT count(*) FROM agent_roles", [], |row| row.get(0))
            .unwrap();
        let catalog_workspaces: i64 = connection
            .query_row("SELECT count(*) FROM workspaces", [], |row| row.get(0))
            .unwrap();
        let catalog_teams: i64 = connection
            .query_row("SELECT count(*) FROM teams", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            (catalog_agents, catalog_workspaces, catalog_teams),
            (0, 0, 0)
        );
    }

    #[test]
    fn v1_migration_preserves_task_run_and_event_history() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        let (mut task, mut run, event);
        {
            let mut store = Store::open(&path).unwrap();
            (task, run) = store
                .create_pending(
                    &request("00000000-0000-4000-8000-000000000021"),
                    "/tmp/synthetic-workspace",
                )
                .unwrap();
            store
                .set_state(&run.id, RunState::Starting, "starting", None, None)
                .unwrap();
            store
                .set_state(&run.id, RunState::Running, "running", Some(52), Some(52))
                .unwrap();
            event = store
                .append_event(&run.id, "text", "preserve this event")
                .unwrap()
                .unwrap();
            store
                .finish(
                    &run.id,
                    RunFinish {
                        run_state: RunState::Crashed,
                        outcome: "synthetic-failure",
                        task_state: "failed",
                        exit_code: Some(7),
                        summary: Some("preserve summary"),
                        error_code: Some("execution.synthetic-failure"),
                        pid: None,
                    },
                )
                .unwrap();
            task = store.get_task(&task.id).unwrap();
            run = store.get_run(&run.id).unwrap();
        }
        downgrade_to_v1(&path);

        let store = Store::open(&path).unwrap();
        let migrated_task = store.get_task(&task.id).unwrap();
        let migrated_run = store.get_run(&run.id).unwrap();
        let migrated_events = store.list_events(&run.id, 0, 10).unwrap();
        let version: i64 = store
            .0
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();

        assert_eq!(version, 3);
        assert_eq!(migrated_task, task);
        assert_eq!(migrated_run, run);
        assert_eq!(migrated_events, vec![event]);
    }

    #[test]
    fn failed_v1_migration_rolls_back_added_columns_and_keeps_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        {
            let _store = Store::open(&path).unwrap();
        }
        downgrade_to_v1(&path);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "ALTER TABLE runs ADD COLUMN recovery_checked_at INTEGER",
                [],
            )
            .unwrap();
        drop(connection);

        let error = match Store::open(&path) {
            Ok(_) => panic!("migration unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error.code, "storage.write-failed");
        let connection = Connection::open(path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let columns = connection
            .prepare("PRAGMA table_info(runs)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(version, 1);
        assert!(!columns.contains(&"process_group_id".into()));
        assert!(!columns.contains(&"recovery_status".into()));
    }

    #[test]
    fn v2_migration_preserves_unknown_run_recovery_fields_and_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        let (history, process_group_id) = {
            let mut store = Store::open(&path).unwrap();
            let mut history = Vec::new();

            let (success_task, success_run) = store
                .create_pending(
                    &request("00000000-0000-4000-8000-000000000032"),
                    "/tmp/catalog-migration",
                )
                .unwrap();
            store
                .set_state(&success_run.id, RunState::Starting, "starting", None, None)
                .unwrap();
            store
                .set_state(
                    &success_run.id,
                    RunState::Running,
                    "running",
                    Some(81),
                    Some(81),
                )
                .unwrap();
            store
                .append_event(&success_run.id, "text", "preserve success")
                .unwrap();
            let success_run = store
                .finish(
                    &success_run.id,
                    RunFinish {
                        run_state: RunState::Finished,
                        outcome: "success",
                        task_state: "done",
                        exit_code: Some(0),
                        summary: Some("successful run"),
                        error_code: None,
                        pid: None,
                    },
                )
                .unwrap();
            history.push((
                store.get_task(&success_task.id).unwrap(),
                success_run.clone(),
                store.list_events(&success_run.id, 0, 10).unwrap(),
            ));

            let (failed_task, failed_run) = store
                .create_pending(
                    &request("00000000-0000-4000-8000-000000000033"),
                    "/tmp/catalog-migration",
                )
                .unwrap();
            store
                .set_state(&failed_run.id, RunState::Starting, "starting", None, None)
                .unwrap();
            store
                .set_state(
                    &failed_run.id,
                    RunState::Running,
                    "running",
                    Some(82),
                    Some(82),
                )
                .unwrap();
            store
                .append_event(&failed_run.id, "text", "preserve failure")
                .unwrap();
            let failed_run = store
                .finish(
                    &failed_run.id,
                    RunFinish {
                        run_state: RunState::Crashed,
                        outcome: "failed",
                        task_state: "failed",
                        exit_code: Some(7),
                        summary: Some("failed run"),
                        error_code: Some("execution.cli-not-ready"),
                        pid: None,
                    },
                )
                .unwrap();
            history.push((
                store.get_task(&failed_task.id).unwrap(),
                failed_run.clone(),
                store.list_events(&failed_run.id, 0, 10).unwrap(),
            ));

            let (created_task, created_run) = store
                .create_pending(
                    &request("00000000-0000-4000-8000-000000000031"),
                    "/tmp/catalog-migration",
                )
                .unwrap();
            store
                .set_state(&created_run.id, RunState::Starting, "starting", None, None)
                .unwrap();
            store
                .set_state(
                    &created_run.id,
                    RunState::Running,
                    "running",
                    Some(83),
                    Some(83),
                )
                .unwrap();
            store
                .append_event(&created_run.id, "text", "preserve before v3")
                .unwrap();
            let unknown = store
                .finish(
                    &created_run.id,
                    RunFinish {
                        run_state: RunState::Unknown,
                        outcome: "unknown",
                        task_state: "unknown",
                        exit_code: None,
                        summary: Some("uncertain process"),
                        error_code: Some("run.stop-unconfirmed"),
                        pid: Some(83),
                    },
                )
                .unwrap();
            history.push((
                store.get_task(&created_task.id).unwrap(),
                unknown,
                store.list_events(&created_run.id, 0, 10).unwrap(),
            ));
            let process_group_id = store.process_group_id(&created_run.id).unwrap();
            (history, process_group_id)
        };
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("DROP TABLE team_members; DROP TABLE teams; DROP TABLE workspace_scopes; DROP TABLE workspaces; DROP TABLE agent_permission_configs; DROP TABLE agent_roles; DROP TABLE catalog_audit; DROP TABLE catalog_settings; PRAGMA user_version=2;").unwrap();
        drop(connection);

        let store = Store::open(&path).unwrap();
        for (task, run, events) in history {
            assert_eq!(store.get_task(&task.id).unwrap(), task);
            assert_eq!(store.get_run(&run.id).unwrap(), run);
            assert_eq!(store.list_events(&run.id, 0, 10).unwrap(), events);
            if run.state == RunState::Unknown {
                assert_eq!(store.process_group_id(&run.id).unwrap(), process_group_id);
            }
        }
        let version: i64 = store
            .0
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn failed_v2_catalog_migration_rolls_back_and_preserves_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        {
            let _store = Store::open(&path).unwrap();
        }
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("DROP TABLE team_members; DROP TABLE teams; DROP TABLE workspace_scopes; DROP TABLE workspaces; DROP TABLE agent_permission_configs; DROP TABLE catalog_audit; DROP TABLE catalog_settings; PRAGMA user_version=2;").unwrap();
        drop(connection);

        let error = match Store::open(&path) {
            Ok(_) => panic!("catalog migration unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error.code, "storage.write-failed");
        let connection = Connection::open(path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='agent_roles'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 2);
        assert_eq!(tables, 1); // the pre-existing conflicting table is retained
        let partial_tables: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('agent_permission_configs','workspaces','workspace_scopes','teams','team_members','catalog_audit','catalog_settings')", [], |row| row.get(0)).unwrap();
        assert_eq!(partial_tables, 0);
    }

    #[test]
    fn future_schema_is_rejected_without_modification() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        {
            let _store = Store::open(&path).unwrap();
        }
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", 4)
            .unwrap();

        let error = match Store::open(&path) {
            Ok(_) => panic!("future schema unexpectedly opened"),
            Err(error) => error,
        };
        assert_eq!(error.code, "storage.schema-unsupported");
        let connection = Connection::open(path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);
    }

    #[test]
    fn startup_recovery_preserves_original_error_and_adds_audit_event() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let (task, run) = store
            .create_pending(
                &request("00000000-0000-4000-8000-000000000022"),
                "/tmp/synthetic-workspace",
            )
            .unwrap();
        store
            .set_state(&run.id, RunState::Starting, "starting", None, None)
            .unwrap();
        store
            .set_state(&run.id, RunState::Running, "running", Some(59), Some(59))
            .unwrap();
        store
            .0
            .execute(
                "UPDATE runs SET error_code='execution.original-failure' WHERE id=?1",
                [&run.id],
            )
            .unwrap();

        store.recover_active().unwrap();
        let recovered = store.get_run(&run.id).unwrap();
        let events = store.list_events(&run.id, 0, 10).unwrap();

        assert_eq!(recovered.state, RunState::Unknown);
        assert_eq!(
            recovered.error_code.as_deref(),
            Some("execution.original-failure")
        );
        assert_eq!(recovered.recovery_status, RecoveryStatus::Required);
        assert_eq!(store.get_task(&task.id).unwrap().state, "unknown");
        assert!(events.iter().any(|event| event.kind == "recovery-required"));
    }
}
