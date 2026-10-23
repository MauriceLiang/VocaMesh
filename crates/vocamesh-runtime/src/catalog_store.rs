use crate::store::{new_id, now, Store};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Component, Path, PathBuf},
    str::FromStr,
};
use vocamesh_core::{
    AddTeamMemberInput, AgentPermissions, AgentRole, CatalogStatus, CreateAgentInput,
    CreateTeamInput, CreateWorkspaceInput, Provider, ServiceError, TeamMemberRecord, TeamRecord,
    UpdateAgentInput, UpdateTeamInput, UpdateWorkspaceScopesInput, WorkspaceRecord,
    WorkspaceScopes,
};

const ERR_READ: ServiceError = ServiceError::new("storage.read-failed", "本地配置无法读取");
const ERR_WRITE: ServiceError = ServiceError::new("storage.write-failed", "本地配置无法保存");
const ERR_AGENT_NOT_FOUND: ServiceError =
    ServiceError::new("agent.not-found", "找不到该 Agent 角色");
const ERR_AGENT_DUPLICATE: ServiceError = ServiceError::new("agent.duplicate", "Agent 名称已存在");
const ERR_AGENT_INVALID: ServiceError = ServiceError::new(
    "agent.invalid-input",
    "Agent 配置无效，请检查名称、职责和工具",
);
const ERR_AGENT_IN_USE: ServiceError =
    ServiceError::new("agent.in-use", "该 Agent 仍属于团队，请先调整团队成员关系");
const ERR_AGENT_REVISION: ServiceError = ServiceError::new(
    "agent.revision-conflict",
    "Agent 配置已被其他窗口修改，请重新加载后再保存",
);
const ERR_WORKSPACE_NOT_FOUND: ServiceError =
    ServiceError::new("workspace.not-found", "找不到该工作空间");
const ERR_WORKSPACE_INVALID: ServiceError =
    ServiceError::new("workspace.invalid-input", "工作空间名称无效");
const ERR_WORKSPACE_DUPLICATE: ServiceError =
    ServiceError::new("workspace.duplicate", "工作空间名称或目录已登记");
const ERR_WORKSPACE_PATH_MISSING: ServiceError =
    ServiceError::new("workspace.path-missing", "工作空间目录不存在或无法访问");
const ERR_WORKSPACE_PATH_INVALID: ServiceError =
    ServiceError::new("workspace.invalid-path", "工作空间必须是安全的本地目录");
const ERR_WORKSPACE_SCOPE: ServiceError = ServiceError::new(
    "workspace.invalid-scope",
    "工作空间范围必须是安全的相对路径",
);
const ERR_WORKSPACE_REVISION: ServiceError = ServiceError::new(
    "workspace.revision-conflict",
    "工作空间已被其他窗口修改，请重新加载后再保存",
);
const ERR_WORKSPACE_IN_USE: ServiceError = ServiceError::new(
    "workspace.in-use",
    "该工作空间仍被团队引用，请先调整团队设置",
);
const ERR_TEAM_NOT_FOUND: ServiceError = ServiceError::new("team.not-found", "找不到该团队");
const ERR_TEAM_DUPLICATE: ServiceError = ServiceError::new("team.duplicate", "团队名称已存在");
const ERR_TEAM_INVALID: ServiceError = ServiceError::new("team.invalid-input", "团队名称无效");
const ERR_TEAM_MEMBER_EXISTS: ServiceError =
    ServiceError::new("team.member-exists", "该 Agent 已在团队中");
const ERR_TEAM_MEMBER_NOT_FOUND: ServiceError =
    ServiceError::new("team.member-not-found", "该 Agent 不是团队成员");
const ERR_TEAM_SELF: ServiceError =
    ServiceError::new("team.self-management", "Agent 不能向自己汇报");
const ERR_TEAM_CYCLE: ServiceError = ServiceError::new("team.cycle", "该汇报关系会形成循环");
const ERR_TEAM_SUPERVISOR: ServiceError = ServiceError::new(
    "team.supervisor-required",
    "团队必须保留一名主管，请先指定新的主管",
);
const ERR_TEAM_WORKSPACE: ServiceError =
    ServiceError::new("team.workspace-unavailable", "请先登记有效工作空间");
const ERR_TEAM_REVISION: ServiceError = ServiceError::new(
    "team.revision-conflict",
    "团队已被其他窗口修改，请重新加载后再保存",
);
const ERR_SETTINGS_INVALID: ServiceError = ServiceError::new("settings.invalid", "外观设置值无效");

impl Store {
    pub(crate) fn catalog_status(&self) -> Result<CatalogStatus, ServiceError> {
        Ok(CatalogStatus {
            schema_version: 3,
            agents: count(&self.0, "agent_roles")?,
            workspaces: count(&self.0, "workspaces")?,
            teams: count(&self.0, "teams")?,
        })
    }

    pub(crate) fn list_agents(&self) -> Result<Vec<AgentRole>, ServiceError> {
        let mut statement = self
            .0
            .prepare("SELECT id FROM agent_roles WHERE archived_at IS NULL ORDER BY created_at,id")
            .map_err(|_| ERR_READ)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        ids.iter().map(|id| get_agent(&self.0, id)).collect()
    }

    pub(crate) fn get_agent(&self, id: &str) -> Result<AgentRole, ServiceError> {
        get_agent(&self.0, id)
    }

    pub(crate) fn create_agent(
        &mut self,
        input: CreateAgentInput,
    ) -> Result<AgentRole, ServiceError> {
        let name = validate_name(&input.name, ERR_AGENT_INVALID)?;
        if Provider::from_str(&input.provider_id).is_err()
            || input.role.trim().len() > 120
            || input.description.len() > 8_000
            || input.instructions.len() > 32_000
        {
            return Err(ERR_AGENT_INVALID);
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let normalized = normalize_name(&name);
        if exists(
            &tx,
            "SELECT 1 FROM agent_roles WHERE name_normalized=?1",
            [&normalized],
        )? {
            return Err(ERR_AGENT_DUPLICATE);
        }
        let id = new_id(&tx, "agent")?;
        let timestamp = now();
        tx.execute("INSERT INTO agent_roles(id,name,name_normalized,role,provider_id,description,instructions,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8)", params![id,name,normalized,input.role.trim(),input.provider_id,input.description,input.instructions,timestamp]).map_err(|_| ERR_WRITE)?;
        insert_permissions(&tx, &id, &input.permissions, timestamp)?;
        audit(&tx, "agent", &id, "created", 1)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_agent(&self.0, &id)
    }

    pub(crate) fn update_agent(
        &mut self,
        id: &str,
        input: UpdateAgentInput,
    ) -> Result<AgentRole, ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let current = get_agent_tx(&tx, id)?;
        if input
            .expected_revision
            .is_some_and(|revision| revision != current.revision)
        {
            return Err(ERR_AGENT_REVISION);
        }
        let name = input
            .name
            .map(|value| validate_name(&value, ERR_AGENT_INVALID))
            .transpose()?
            .unwrap_or(current.name);
        let role = input.role.unwrap_or(current.role);
        let provider = input.provider_id.unwrap_or(current.provider_id);
        let description = input.description.unwrap_or(current.description);
        let instructions = input.instructions.unwrap_or(current.instructions);
        let permissions = input.permissions.unwrap_or(current.permissions);
        if Provider::from_str(&provider).is_err()
            || role.trim().len() > 120
            || description.len() > 8_000
            || instructions.len() > 32_000
        {
            return Err(ERR_AGENT_INVALID);
        }
        let normalized = normalize_name(&name);
        if exists(
            &tx,
            "SELECT 1 FROM agent_roles WHERE name_normalized=?1 AND id<>?2 AND archived_at IS NULL",
            params![normalized, id],
        )? {
            return Err(ERR_AGENT_DUPLICATE);
        }
        let revision = current.revision + 1;
        let timestamp = now();
        tx.execute("UPDATE agent_roles SET name=?2,name_normalized=?3,role=?4,provider_id=?5,description=?6,instructions=?7,revision=?8,updated_at=?9 WHERE id=?1 AND archived_at IS NULL", params![id,name,normalized,role.trim(),provider,description,instructions,revision,timestamp]).map_err(|_| ERR_WRITE)?;
        tx.execute("UPDATE agent_permission_configs SET request_read=?2,request_write=?3,request_shell=?4,request_network=?5,revision=revision+1,updated_at=?6 WHERE agent_id=?1", params![id,permissions.read,permissions.write,permissions.shell,permissions.network,timestamp]).map_err(|_| ERR_WRITE)?;
        audit(&tx, "agent", id, "updated", revision)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_agent(&self.0, id)
    }

    pub(crate) fn archive_agent(&mut self, id: &str) -> Result<(), ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let current = get_agent_tx(&tx, id)?;
        let in_use = exists(&tx, "SELECT 1 FROM team_members m JOIN teams t ON t.id=m.team_id WHERE m.agent_id=?1 AND t.archived_at IS NULL", [id])?;
        if in_use {
            return Err(ERR_AGENT_IN_USE);
        }
        let revision = current.revision + 1;
        let timestamp = now();
        tx.execute("UPDATE agent_roles SET archived_at=?2,revision=?3,updated_at=?2 WHERE id=?1 AND archived_at IS NULL", params![id,timestamp,revision]).map_err(|_| ERR_WRITE)?;
        audit(&tx, "agent", id, "archived", revision)?;
        tx.commit().map_err(|_| ERR_WRITE)
    }

    pub(crate) fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, ServiceError> {
        let mut statement = self
            .0
            .prepare("SELECT id FROM workspaces WHERE archived_at IS NULL ORDER BY created_at,id")
            .map_err(|_| ERR_READ)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        ids.iter().map(|id| get_workspace(&self.0, id)).collect()
    }

    pub(crate) fn get_workspace(&self, id: &str) -> Result<WorkspaceRecord, ServiceError> {
        get_workspace(&self.0, id)
    }

    pub(crate) fn create_workspace(
        &mut self,
        input: CreateWorkspaceInput,
    ) -> Result<WorkspaceRecord, ServiceError> {
        let name = validate_name(&input.name, ERR_WORKSPACE_INVALID)?;
        let canonical_path = validate_workspace_path(&input.path)?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let normalized = normalize_name(&name);
        if exists(
            &tx,
            "SELECT 1 FROM workspaces WHERE name_normalized=?1 AND archived_at IS NULL",
            [&normalized],
        )? || exists(
            &tx,
            "SELECT 1 FROM workspaces WHERE canonical_path=?1 AND archived_at IS NULL",
            [canonical_path.as_str()],
        )? {
            return Err(ERR_WORKSPACE_DUPLICATE);
        }
        let id = new_id(&tx, "workspace")?;
        let timestamp = now();
        tx.execute("INSERT INTO workspaces(id,name,name_normalized,canonical_path,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)", params![id,name,normalized,canonical_path,timestamp]).map_err(|_| ERR_WRITE)?;
        tx.execute("INSERT INTO workspace_scopes(workspace_id,scope_kind,relative_path) VALUES(?1,'read','.')", [&id]).map_err(|_| ERR_WRITE)?;
        audit(&tx, "workspace", &id, "created", 1)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_workspace(&self.0, &id)
    }

    pub(crate) fn update_workspace_scopes(
        &mut self,
        id: &str,
        input: UpdateWorkspaceScopesInput,
    ) -> Result<WorkspaceRecord, ServiceError> {
        let scopes = validate_scopes(input.scopes)?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let current = get_workspace_tx(&tx, id)?;
        if input
            .expected_revision
            .is_some_and(|revision| revision != current.revision)
        {
            return Err(ERR_WORKSPACE_REVISION);
        }
        tx.execute("DELETE FROM workspace_scopes WHERE workspace_id=?1", [id])
            .map_err(|_| ERR_WRITE)?;
        for (kind, paths) in [
            ("read", &scopes.read),
            ("write", &scopes.write),
            ("deny", &scopes.deny),
        ] {
            for path in paths {
                tx.execute("INSERT INTO workspace_scopes(workspace_id,scope_kind,relative_path) VALUES(?1,?2,?3)", params![id,kind,path]).map_err(|_| ERR_WRITE)?;
            }
        }
        let revision = current.revision + 1;
        let timestamp = now();
        tx.execute(
            "UPDATE workspaces SET revision=?2,updated_at=?3 WHERE id=?1 AND archived_at IS NULL",
            params![id, revision, timestamp],
        )
        .map_err(|_| ERR_WRITE)?;
        audit(&tx, "workspace", id, "scopes-updated", revision)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_workspace(&self.0, id)
    }

    pub(crate) fn rename_workspace(
        &mut self,
        id: &str,
        raw_name: &str,
        expected_revision: Option<u64>,
    ) -> Result<WorkspaceRecord, ServiceError> {
        let name = validate_name(raw_name, ERR_WORKSPACE_INVALID)?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let current = get_workspace_tx(&tx, id)?;
        if expected_revision.is_some_and(|revision| revision != current.revision) {
            return Err(ERR_WORKSPACE_REVISION);
        }
        let normalized = normalize_name(&name);
        if exists(
            &tx,
            "SELECT 1 FROM workspaces WHERE name_normalized=?1 AND id<>?2 AND archived_at IS NULL",
            params![normalized, id],
        )? {
            return Err(ERR_WORKSPACE_DUPLICATE);
        }
        let revision = current.revision + 1;
        let timestamp = now();
        tx.execute(
            "UPDATE workspaces SET name=?2,name_normalized=?3,revision=?4,updated_at=?5 WHERE id=?1 AND archived_at IS NULL",
            params![id, name, normalized, revision, timestamp],
        )
        .map_err(|_| ERR_WRITE)?;
        audit(&tx, "workspace", id, "renamed", revision)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_workspace(&self.0, id)
    }

    pub(crate) fn archive_workspace(&mut self, id: &str) -> Result<(), ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let current = get_workspace_tx(&tx, id)?;
        if exists(
            &tx,
            "SELECT 1 FROM teams WHERE workspace_id=?1 AND archived_at IS NULL",
            [id],
        )? {
            return Err(ERR_WORKSPACE_IN_USE);
        }
        let revision = current.revision + 1;
        let timestamp = now();
        tx.execute("UPDATE workspaces SET archived_at=?2,revision=?3,updated_at=?2 WHERE id=?1 AND archived_at IS NULL", params![id,timestamp,revision]).map_err(|_| ERR_WRITE)?;
        audit(&tx, "workspace", id, "archived", revision)?;
        tx.commit().map_err(|_| ERR_WRITE)
    }

    pub(crate) fn list_teams(&self) -> Result<Vec<TeamRecord>, ServiceError> {
        let mut statement = self
            .0
            .prepare("SELECT id FROM teams WHERE archived_at IS NULL ORDER BY created_at,id")
            .map_err(|_| ERR_READ)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| ERR_READ)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ERR_READ)?;
        ids.iter().map(|id| get_team(&self.0, id)).collect()
    }

    pub(crate) fn get_team(&self, id: &str) -> Result<TeamRecord, ServiceError> {
        get_team(&self.0, id)
    }

    pub(crate) fn create_team(
        &mut self,
        input: CreateTeamInput,
    ) -> Result<TeamRecord, ServiceError> {
        let name = validate_name(&input.name, ERR_TEAM_INVALID)?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        get_agent_tx(&tx, &input.supervisor_id).map_err(|_| ERR_TEAM_SUPERVISOR)?;
        let workspace =
            get_workspace_tx(&tx, &input.workspace_id).map_err(|_| ERR_TEAM_WORKSPACE)?;
        if !workspace.path_available {
            return Err(ERR_TEAM_WORKSPACE);
        }
        let normalized = normalize_name(&name);
        if exists(
            &tx,
            "SELECT 1 FROM teams WHERE name_normalized=?1 AND archived_at IS NULL",
            [&normalized],
        )? {
            return Err(ERR_TEAM_DUPLICATE);
        }
        let id = new_id(&tx, "team")?;
        let timestamp = now();
        tx.execute("INSERT INTO teams(id,name,name_normalized,supervisor_agent_id,workspace_id,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?6)", params![id,name,normalized,input.supervisor_id,input.workspace_id,timestamp]).map_err(|_| ERR_WRITE)?;
        tx.execute(
            "INSERT INTO team_members(team_id,agent_id,reports_to_agent_id) VALUES(?1,?2,NULL)",
            params![id, input.supervisor_id],
        )
        .map_err(|_| ERR_WRITE)?;
        audit(&tx, "team", &id, "created", 1)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_team(&self.0, &id)
    }

    pub(crate) fn update_team(
        &mut self,
        id: &str,
        input: UpdateTeamInput,
    ) -> Result<TeamRecord, ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let current = get_team_tx(&tx, id)?;
        if input
            .expected_revision
            .is_some_and(|revision| revision != current.revision)
        {
            return Err(ERR_TEAM_REVISION);
        }
        let name = input
            .name
            .map(|value| validate_name(&value, ERR_TEAM_INVALID))
            .transpose()?
            .unwrap_or(current.name);
        let supervisor = input
            .supervisor_id
            .unwrap_or_else(|| current.supervisor_id.clone());
        let workspace = input.workspace_id.unwrap_or(current.workspace_id);
        if get_agent_tx(&tx, &supervisor).is_err() {
            return Err(ERR_TEAM_SUPERVISOR);
        }
        let workspace_record = get_workspace_tx(&tx, &workspace).map_err(|_| ERR_TEAM_WORKSPACE)?;
        if !workspace_record.path_available {
            return Err(ERR_TEAM_WORKSPACE);
        }
        let normalized = normalize_name(&name);
        if exists(
            &tx,
            "SELECT 1 FROM teams WHERE name_normalized=?1 AND id<>?2 AND archived_at IS NULL",
            params![normalized, id],
        )? {
            return Err(ERR_TEAM_DUPLICATE);
        }
        if supervisor != current.supervisor_id {
            transfer_supervisor(&tx, id, &current.supervisor_id, &supervisor)?;
        }
        let revision = current.revision + 1;
        let timestamp = now();
        tx.execute("UPDATE teams SET name=?2,name_normalized=?3,supervisor_agent_id=?4,workspace_id=?5,revision=?6,updated_at=?7 WHERE id=?1 AND archived_at IS NULL", params![id,name,normalized,supervisor,workspace,revision,timestamp]).map_err(|_| ERR_WRITE)?;
        validate_team_tree(&tx, id)?;
        audit(&tx, "team", id, "updated", revision)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_team(&self.0, id)
    }

    pub(crate) fn add_team_member(
        &mut self,
        team_id: &str,
        input: AddTeamMemberInput,
    ) -> Result<TeamRecord, ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let team = get_team_tx(&tx, team_id)?;
        get_agent_tx(&tx, &input.agent_id).map_err(|_| ERR_TEAM_MEMBER_NOT_FOUND)?;
        if input.agent_id == team.supervisor_id {
            return Err(ERR_TEAM_SELF);
        }
        if exists(
            &tx,
            "SELECT 1 FROM team_members WHERE team_id=?1 AND agent_id=?2",
            params![team_id, input.agent_id],
        )? {
            return Err(ERR_TEAM_MEMBER_EXISTS);
        }
        if input.reports_to_agent_id == input.agent_id {
            return Err(ERR_TEAM_SELF);
        }
        if input.reports_to_agent_id == "user"
            || !exists(
                &tx,
                "SELECT 1 FROM team_members WHERE team_id=?1 AND agent_id=?2",
                params![team_id, input.reports_to_agent_id],
            )?
        {
            return Err(ERR_TEAM_MEMBER_NOT_FOUND);
        }
        tx.execute(
            "INSERT INTO team_members(team_id,agent_id,reports_to_agent_id) VALUES(?1,?2,?3)",
            params![team_id, input.agent_id, input.reports_to_agent_id],
        )
        .map_err(|_| ERR_WRITE)?;
        bump_team(&tx, team_id, "member-added")?;
        validate_team_tree(&tx, team_id)?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_team(&self.0, team_id)
    }

    pub(crate) fn remove_team_member(
        &mut self,
        team_id: &str,
        agent_id: &str,
    ) -> Result<TeamRecord, ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let team = get_team_tx(&tx, team_id)?;
        if agent_id == team.supervisor_id {
            return Err(ERR_TEAM_SUPERVISOR);
        }
        if !exists(
            &tx,
            "SELECT 1 FROM team_members WHERE team_id=?1 AND agent_id=?2",
            params![team_id, agent_id],
        )? {
            return Err(ERR_TEAM_MEMBER_NOT_FOUND);
        }
        if exists(
            &tx,
            "SELECT 1 FROM team_members WHERE team_id=?1 AND reports_to_agent_id=?2",
            params![team_id, agent_id],
        )? {
            return Err(ERR_TEAM_SUPERVISOR);
        }
        tx.execute(
            "DELETE FROM team_members WHERE team_id=?1 AND agent_id=?2",
            params![team_id, agent_id],
        )
        .map_err(|_| ERR_WRITE)?;
        bump_team(&tx, team_id, "member-removed")?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_team(&self.0, team_id)
    }

    pub(crate) fn set_team_reports_to(
        &mut self,
        team_id: &str,
        agent_id: &str,
        reports_to: &str,
    ) -> Result<TeamRecord, ServiceError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        let team = get_team_tx(&tx, team_id)?;
        if !exists(
            &tx,
            "SELECT 1 FROM team_members WHERE team_id=?1 AND agent_id=?2",
            params![team_id, agent_id],
        )? {
            return Err(ERR_TEAM_MEMBER_NOT_FOUND);
        }
        if reports_to == agent_id {
            return Err(ERR_TEAM_SELF);
        }
        if reports_to == "user" {
            if agent_id != team.supervisor_id {
                tx.execute("UPDATE team_members SET reports_to_agent_id=?3 WHERE team_id=?1 AND agent_id=?2", params![team_id,team.supervisor_id,agent_id]).map_err(|_| ERR_WRITE)?;
                tx.execute("UPDATE team_members SET reports_to_agent_id=NULL WHERE team_id=?1 AND agent_id=?2", params![team_id,agent_id]).map_err(|_| ERR_WRITE)?;
                tx.execute(
                    "UPDATE teams SET supervisor_agent_id=?2 WHERE id=?1",
                    params![team_id, agent_id],
                )
                .map_err(|_| ERR_WRITE)?;
            }
        } else {
            if !exists(
                &tx,
                "SELECT 1 FROM team_members WHERE team_id=?1 AND agent_id=?2",
                params![team_id, reports_to],
            )? {
                return Err(ERR_TEAM_MEMBER_NOT_FOUND);
            }
            if agent_id == team.supervisor_id {
                // Selecting a new parent transfers supervision and keeps one root.
                tx.execute("UPDATE team_members SET reports_to_agent_id=?3 WHERE team_id=?1 AND agent_id=?2", params![team_id,team.supervisor_id,reports_to]).map_err(|_| ERR_WRITE)?;
                tx.execute("UPDATE team_members SET reports_to_agent_id=NULL WHERE team_id=?1 AND agent_id=?2", params![team_id,reports_to]).map_err(|_| ERR_WRITE)?;
                tx.execute(
                    "UPDATE teams SET supervisor_agent_id=?2 WHERE id=?1",
                    params![team_id, reports_to],
                )
                .map_err(|_| ERR_WRITE)?;
            } else {
                tx.execute("UPDATE team_members SET reports_to_agent_id=?3 WHERE team_id=?1 AND agent_id=?2", params![team_id,agent_id,reports_to]).map_err(|_| ERR_WRITE)?;
            }
        }
        validate_team_tree(&tx, team_id)?;
        bump_team(&tx, team_id, "reporting-updated")?;
        tx.commit().map_err(|_| ERR_WRITE)?;
        get_team(&self.0, team_id)
    }

    pub(crate) fn get_catalog_setting(&self, key: &str) -> Result<Option<String>, ServiceError> {
        if !matches!(key, "appearance") {
            return Err(ERR_SETTINGS_INVALID);
        }
        self.0
            .query_row(
                "SELECT value FROM catalog_settings WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ERR_READ)
    }

    pub(crate) fn set_catalog_setting(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), ServiceError> {
        if !matches!(key, "appearance") || value.len() > 4_096 {
            return Err(ERR_SETTINGS_INVALID);
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ERR_WRITE)?;
        tx.execute("INSERT INTO catalog_settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value]).map_err(|_| ERR_WRITE)?;
        audit(&tx, "settings", "appearance", "updated", 1)?;
        tx.commit().map_err(|_| ERR_WRITE)
    }
}

fn count(connection: &rusqlite::Connection, table: &str) -> Result<u64, ServiceError> {
    let sql = match table {
        "agent_roles" => "SELECT count(*) FROM agent_roles WHERE archived_at IS NULL",
        "workspaces" => "SELECT count(*) FROM workspaces WHERE archived_at IS NULL",
        _ => "SELECT count(*) FROM teams WHERE archived_at IS NULL",
    };
    connection
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count as u64)
        .map_err(|_| ERR_READ)
}

fn exists<P: rusqlite::Params, C: ConnectionLike>(
    connection: &C,
    sql: &str,
    params: P,
) -> Result<bool, ServiceError> {
    connection.exists_query(sql, params)
}

trait ConnectionLike {
    fn exists_query<P: rusqlite::Params>(&self, sql: &str, params: P)
        -> Result<bool, ServiceError>;
}

impl ConnectionLike for rusqlite::Connection {
    fn exists_query<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<bool, ServiceError> {
        self.query_row(sql, params, |_| Ok(()))
            .optional()
            .map(|value| value.is_some())
            .map_err(|_| ERR_READ)
    }
}

impl ConnectionLike for Transaction<'_> {
    fn exists_query<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<bool, ServiceError> {
        self.query_row(sql, params, |_| Ok(()))
            .optional()
            .map(|value| value.is_some())
            .map_err(|_| ERR_READ)
    }
}

fn validate_name(value: &str, error: ServiceError) -> Result<String, ServiceError> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(error);
    }
    Ok(name.to_owned())
}

fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn validate_workspace_path(value: &str) -> Result<String, ServiceError> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(ERR_WORKSPACE_PATH_INVALID);
    }
    let canonical = fs::canonicalize(path).map_err(|_| ERR_WORKSPACE_PATH_MISSING)?;
    if !canonical.is_dir() {
        return Err(ERR_WORKSPACE_PATH_INVALID);
    }
    let mut cursor = PathBuf::new();
    for component in path.components() {
        cursor.push(component.as_os_str());
        if fs::symlink_metadata(&cursor).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(ERR_WORKSPACE_PATH_INVALID);
        }
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .and_then(|path| fs::canonicalize(path).ok());
    let forbidden_exact = [
        "/",
        "/Users",
        "/private/tmp",
        "/tmp",
        "/private/var",
        "/var",
    ];
    if forbidden_exact
        .iter()
        .any(|blocked| canonical == Path::new(blocked))
        || home.as_deref() == Some(canonical.as_path())
    {
        return Err(ERR_WORKSPACE_PATH_INVALID);
    }
    let forbidden_trees = [
        "/System",
        "/Library",
        "/Applications",
        "/usr",
        "/bin",
        "/sbin",
        "/dev",
        "/proc",
        "/etc",
    ];
    if forbidden_trees
        .iter()
        .any(|blocked| canonical.starts_with(blocked))
    {
        return Err(ERR_WORKSPACE_PATH_INVALID);
    }
    Ok(canonical.to_string_lossy().into_owned())
}

fn validate_scopes(scopes: WorkspaceScopes) -> Result<WorkspaceScopes, ServiceError> {
    fn clean(items: Vec<String>) -> Result<Vec<String>, ServiceError> {
        let mut result = BTreeSet::new();
        for item in items {
            if item == "." {
                result.insert(item);
                continue;
            }
            if item.is_empty()
                || item.starts_with('/')
                || item.contains('\\')
                || item.chars().any(char::is_control)
            {
                return Err(ERR_WORKSPACE_SCOPE);
            }
            let components: Vec<_> = item.split('/').collect();
            if components
                .iter()
                .any(|part| part.is_empty() || *part == "." || *part == "..")
            {
                return Err(ERR_WORKSPACE_SCOPE);
            }
            if Path::new(&item)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(ERR_WORKSPACE_SCOPE);
            }
            result.insert(item);
        }
        Ok(result.into_iter().collect())
    }
    Ok(WorkspaceScopes {
        read: clean(scopes.read)?,
        write: clean(scopes.write)?,
        deny: clean(scopes.deny)?,
    })
}

fn insert_permissions(
    tx: &Transaction<'_>,
    id: &str,
    permissions: &AgentPermissions,
    timestamp: i64,
) -> Result<(), ServiceError> {
    tx.execute("INSERT INTO agent_permission_configs(agent_id,request_read,request_write,request_shell,request_network,updated_at) VALUES(?1,?2,?3,?4,?5,?6)", params![id,permissions.read,permissions.write,permissions.shell,permissions.network,timestamp]).map_err(|_| ERR_WRITE)?;
    Ok(())
}

fn audit(
    tx: &Transaction<'_>,
    object_type: &str,
    object_id: &str,
    action: &str,
    revision: u64,
) -> Result<(), ServiceError> {
    let id = new_id(tx, "audit")?;
    tx.execute("INSERT INTO catalog_audit(id,object_type,object_id,action,revision,happened_at) VALUES(?1,?2,?3,?4,?5,?6)", params![id,object_type,object_id,action,revision as i64,now()]).map_err(|_| ERR_WRITE)?;
    Ok(())
}

fn get_agent(connection: &rusqlite::Connection, id: &str) -> Result<AgentRole, ServiceError> {
    get_agent_query(connection, id)
}
fn get_agent_tx(tx: &Transaction<'_>, id: &str) -> Result<AgentRole, ServiceError> {
    get_agent_query(tx, id)
}
fn get_agent_query(connection: &Connection, id: &str) -> Result<AgentRole, ServiceError> {
    let mut agent = connection.query_row("SELECT id,name,role,provider_id,description,instructions,revision,created_at,updated_at FROM agent_roles WHERE id=?1 AND archived_at IS NULL", [id], |row| Ok(AgentRole { id: row.get(0)?, name: row.get(1)?, role: row.get(2)?, provider_id: row.get(3)?, description: row.get(4)?, instructions: row.get(5)?, permissions: AgentPermissions::default(), revision: row.get::<_, i64>(6)? as u64, created_at: row.get::<_, i64>(7)? as u64, updated_at: row.get::<_, i64>(8)? as u64 })).optional().map_err(|_| ERR_READ)?.ok_or(ERR_AGENT_NOT_FOUND)?;
    agent.permissions = connection.query_row("SELECT request_read,request_write,request_shell,request_network FROM agent_permission_configs WHERE agent_id=?1", [id], |row| Ok(AgentPermissions { read: row.get(0)?, write: row.get(1)?, shell: row.get(2)?, network: row.get(3)? })).map_err(|_| ERR_READ)?;
    Ok(agent)
}

fn get_workspace(
    connection: &rusqlite::Connection,
    id: &str,
) -> Result<WorkspaceRecord, ServiceError> {
    get_workspace_query(connection, id)
}
fn get_workspace_tx(tx: &Transaction<'_>, id: &str) -> Result<WorkspaceRecord, ServiceError> {
    get_workspace_query(tx, id)
}
fn get_workspace_query(connection: &Connection, id: &str) -> Result<WorkspaceRecord, ServiceError> {
    let (name,path,revision,created_at,updated_at) = connection.query_row("SELECT name,canonical_path,revision,created_at,updated_at FROM workspaces WHERE id=?1 AND archived_at IS NULL", [id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)? as u64,row.get::<_,i64>(3)? as u64,row.get::<_,i64>(4)? as u64))).optional().map_err(|_| ERR_READ)?.ok_or(ERR_WORKSPACE_NOT_FOUND)?;
    let path_available = validate_workspace_path(&path).is_ok();
    let task_count = connection
        .query_row(
            "SELECT count(*) FROM tasks WHERE workspace_path=?1",
            [&path],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| ERR_READ)? as u64;
    let mut scopes = WorkspaceScopes {
        read: Vec::new(),
        write: Vec::new(),
        deny: Vec::new(),
    };
    let mut statement = connection.prepare("SELECT scope_kind,relative_path FROM workspace_scopes WHERE workspace_id=?1 ORDER BY scope_kind,relative_path").map_err(|_| ERR_READ)?;
    let rows = statement
        .query_map([id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| ERR_READ)?;
    for row in rows {
        let (kind, path) = row.map_err(|_| ERR_READ)?;
        match kind.as_str() {
            "read" => scopes.read.push(path),
            "write" => scopes.write.push(path),
            "deny" => scopes.deny.push(path),
            _ => return Err(ERR_READ),
        }
    }
    Ok(WorkspaceRecord {
        id: id.to_owned(),
        name,
        path,
        path_available,
        task_count,
        scopes,
        revision,
        created_at,
        updated_at,
    })
}

fn get_team(connection: &rusqlite::Connection, id: &str) -> Result<TeamRecord, ServiceError> {
    get_team_query(connection, id)
}
fn get_team_tx(tx: &Transaction<'_>, id: &str) -> Result<TeamRecord, ServiceError> {
    get_team_query(tx, id)
}
fn get_team_query(connection: &Connection, id: &str) -> Result<TeamRecord, ServiceError> {
    let (name,supervisor_id,workspace_id,workspace,revision,created_at,updated_at) = connection.query_row("SELECT t.name,t.supervisor_agent_id,t.workspace_id,w.name,t.revision,t.created_at,t.updated_at FROM teams t JOIN workspaces w ON w.id=t.workspace_id WHERE t.id=?1 AND t.archived_at IS NULL AND w.archived_at IS NULL", [id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,i64>(4)? as u64,row.get::<_,i64>(5)? as u64,row.get::<_,i64>(6)? as u64))).optional().map_err(|_| ERR_READ)?.ok_or(ERR_TEAM_NOT_FOUND)?;
    let mut statement = connection.prepare("SELECT m.agent_id,a.name,a.role,a.provider_id,m.reports_to_agent_id FROM team_members m JOIN agent_roles a ON a.id=m.agent_id WHERE m.team_id=?1 ORDER BY CASE WHEN m.agent_id=?2 THEN 0 ELSE 1 END,a.created_at").map_err(|_| ERR_READ)?;
    let members = statement
        .query_map(params![id, supervisor_id], |row| {
            Ok(TeamMemberRecord {
                agent_id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                provider_id: row.get(3)?,
                reports_to_agent_id: row.get(4)?,
            })
        })
        .map_err(|_| ERR_READ)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ERR_READ)?;
    Ok(TeamRecord {
        id: id.to_owned(),
        name,
        supervisor_id,
        workspace_id,
        workspace,
        members,
        revision,
        created_at,
        updated_at,
    })
}

fn transfer_supervisor(
    tx: &Transaction<'_>,
    team_id: &str,
    old: &str,
    new: &str,
) -> Result<(), ServiceError> {
    if !exists(
        tx,
        "SELECT 1 FROM team_members WHERE team_id=?1 AND agent_id=?2",
        params![team_id, new],
    )? {
        return Err(ERR_TEAM_SUPERVISOR);
    }
    if old != new {
        tx.execute(
            "UPDATE team_members SET reports_to_agent_id=?3 WHERE team_id=?1 AND agent_id=?2",
            params![team_id, old, new],
        )
        .map_err(|_| ERR_WRITE)?;
        tx.execute(
            "UPDATE team_members SET reports_to_agent_id=NULL WHERE team_id=?1 AND agent_id=?2",
            params![team_id, new],
        )
        .map_err(|_| ERR_WRITE)?;
    }
    Ok(())
}

fn bump_team(tx: &Transaction<'_>, team_id: &str, action: &str) -> Result<(), ServiceError> {
    let revision: i64 = tx
        .query_row(
            "SELECT revision FROM teams WHERE id=?1 AND archived_at IS NULL",
            [team_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| ERR_READ)?
        .ok_or(ERR_TEAM_NOT_FOUND)?;
    let next = revision + 1;
    tx.execute(
        "UPDATE teams SET revision=?2,updated_at=?3 WHERE id=?1",
        params![team_id, next, now()],
    )
    .map_err(|_| ERR_WRITE)?;
    audit(tx, "team", team_id, action, next as u64)
}

fn validate_team_tree(tx: &Transaction<'_>, team_id: &str) -> Result<(), ServiceError> {
    let supervisor: String = tx
        .query_row(
            "SELECT supervisor_agent_id FROM teams WHERE id=?1",
            [team_id],
            |row| row.get(0),
        )
        .map_err(|_| ERR_TEAM_NOT_FOUND)?;
    let mut statement = tx
        .prepare("SELECT agent_id,reports_to_agent_id FROM team_members WHERE team_id=?1")
        .map_err(|_| ERR_READ)?;
    let rows = statement
        .query_map([team_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|_| ERR_READ)?;
    let mut parents = HashMap::new();
    for row in rows {
        let (agent, parent) = row.map_err(|_| ERR_READ)?;
        parents.insert(agent, parent);
    }
    if parents.get(&supervisor) != Some(&None)
        || parents.values().filter(|parent| parent.is_none()).count() != 1
    {
        return Err(ERR_TEAM_SUPERVISOR);
    }
    for (agent, parent) in &parents {
        if let Some(parent) = parent {
            if parent == agent {
                return Err(ERR_TEAM_SELF);
            }
            if !parents.contains_key(parent) {
                return Err(ERR_TEAM_MEMBER_NOT_FOUND);
            }
        }
        let mut seen = BTreeSet::new();
        let mut cursor = agent.as_str();
        while let Some(Some(parent)) = parents.get(cursor) {
            if !seen.insert(cursor.to_owned()) || parent == agent {
                return Err(ERR_TEAM_CYCLE);
            }
            cursor = parent;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_agent(
        store: &mut Store,
        name: &str,
        provider_id: &str,
    ) -> Result<AgentRole, ServiceError> {
        store.create_agent(CreateAgentInput {
            name: name.into(),
            role: "分析员".into(),
            provider_id: provider_id.into(),
            description: "合成描述".into(),
            instructions: "合成指令".into(),
            permissions: AgentPermissions::default(),
        })
    }

    fn create_workspace(store: &mut Store, parent: &Path, name: &str) -> WorkspaceRecord {
        let directory = parent.join(name.replace(' ', "-"));
        fs::create_dir_all(&directory).unwrap();
        let canonical = fs::canonicalize(directory).unwrap();
        store
            .create_workspace(CreateWorkspaceInput {
                name: name.into(),
                path: canonical.to_string_lossy().into_owned(),
            })
            .unwrap()
    }

    #[test]
    fn catalog_entities_relationships_scopes_and_appearance_survive_store_reopen() {
        let dir = tempdir().unwrap();
        let database = dir.path().join("catalog.sqlite3");
        let (agent, member, workspace, team);
        {
            let mut store = Store::open(&database).unwrap();
            agent = create_agent(&mut store, "Persistent lead", "claude").unwrap();
            member = create_agent(&mut store, "Persistent member", "codex").unwrap();
            workspace = create_workspace(&mut store, dir.path(), "persistent workspace");
            team = store
                .create_team(CreateTeamInput {
                    name: "Persistent team".into(),
                    supervisor_id: agent.id.clone(),
                    workspace_id: workspace.id.clone(),
                })
                .unwrap();
            store
                .add_team_member(
                    &team.id,
                    AddTeamMemberInput {
                        agent_id: member.id.clone(),
                        reports_to_agent_id: agent.id.clone(),
                    },
                )
                .unwrap();
            store
                .update_workspace_scopes(
                    &workspace.id,
                    UpdateWorkspaceScopesInput {
                        scopes: WorkspaceScopes {
                            read: vec!["docs".into()],
                            write: vec!["drafts".into()],
                            deny: vec!["secrets".into()],
                        },
                        expected_revision: Some(workspace.revision),
                    },
                )
                .unwrap();
            store
                .set_catalog_setting(
                    "appearance",
                    r#"{"theme":"dark","density":"compact","grayscalePreview":false,"reducedMotion":true}"#,
                )
                .unwrap();
        }

        let reopened = Store::open(&database).unwrap();
        assert_eq!(reopened.get_agent(&agent.id).unwrap(), agent);
        assert_eq!(reopened.get_agent(&member.id).unwrap(), member);
        let restored_workspace = reopened.get_workspace(&workspace.id).unwrap();
        assert_eq!(restored_workspace.name, workspace.name);
        assert_eq!(restored_workspace.path, workspace.path);
        assert!(restored_workspace.path_available);
        assert_eq!(restored_workspace.scopes.read, vec!["docs"]);
        assert_eq!(restored_workspace.scopes.write, vec!["drafts"]);
        assert_eq!(restored_workspace.scopes.deny, vec!["secrets"]);
        let restored_team = reopened.get_team(&team.id).unwrap();
        assert_eq!(restored_team.workspace_id, workspace.id);
        assert_eq!(restored_team.members.len(), 2);
        assert_eq!(restored_team.members[1].agent_id, member.id);
        assert_eq!(
            restored_team.members[1].reports_to_agent_id.as_deref(),
            Some(agent.id.as_str())
        );
        assert!(reopened
            .get_catalog_setting("appearance")
            .unwrap()
            .unwrap()
            .contains("\"theme\":\"dark\""));
    }

    #[test]
    fn agent_names_providers_permissions_and_archive_references_are_enforced() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite3")).unwrap();
        let agent = create_agent(&mut store, " Analyst ", "codex").unwrap();
        assert_eq!(agent.name, "Analyst");
        assert!(agent.permissions.read);
        assert!(!agent.permissions.write && !agent.permissions.shell && !agent.permissions.network);
        assert_eq!(
            create_agent(&mut store, "analyst", "claude")
                .unwrap_err()
                .code,
            "agent.duplicate"
        );
        assert_eq!(
            create_agent(&mut store, "Unsupported", "omni")
                .unwrap_err()
                .code,
            "agent.invalid-input"
        );
        assert_eq!(
            create_agent(&mut store, "   ", "claude").unwrap_err().code,
            "agent.invalid-input"
        );

        let workspace = create_workspace(&mut store, dir.path(), "main");
        let team = store
            .create_team(CreateTeamInput {
                name: "Team".into(),
                supervisor_id: agent.id.clone(),
                workspace_id: workspace.id,
            })
            .unwrap();
        let renamed = store
            .update_agent(
                &agent.id,
                UpdateAgentInput {
                    name: Some("Renamed Analyst".into()),
                    expected_revision: Some(agent.revision),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(renamed.id, agent.id);
        let renamed_team = store.get_team(&team.id).unwrap();
        assert_eq!(renamed_team.members[0].agent_id, agent.id);
        assert_eq!(renamed_team.members[0].name, "Renamed Analyst");
        assert_eq!(
            store.archive_agent(&agent.id).unwrap_err().code,
            "agent.in-use"
        );
        assert_eq!(team.members.len(), 1);
        assert_eq!(team.members[0].reports_to_agent_id, None);
        assert_eq!(store.catalog_status().unwrap().agents, 1);
    }

    #[test]
    fn workspace_paths_scopes_duplicates_and_archive_references_are_enforced() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite3")).unwrap();
        let workspace = create_workspace(&mut store, dir.path(), "source");
        assert!(workspace.path_available);
        assert_eq!(
            store
                .create_workspace(CreateWorkspaceInput {
                    name: "other".into(),
                    path: workspace.path.clone()
                })
                .unwrap_err()
                .code,
            "workspace.duplicate"
        );
        assert_eq!(
            store
                .update_workspace_scopes(
                    &workspace.id,
                    UpdateWorkspaceScopesInput {
                        scopes: WorkspaceScopes {
                            read: vec!["../secret".into()],
                            write: vec![],
                            deny: vec![]
                        },
                        expected_revision: None,
                    }
                )
                .unwrap_err()
                .code,
            "workspace.invalid-scope"
        );
        assert_eq!(
            store
                .update_workspace_scopes(
                    &workspace.id,
                    UpdateWorkspaceScopesInput {
                        scopes: WorkspaceScopes {
                            read: vec!["/outside".into()],
                            write: vec![],
                            deny: vec![]
                        },
                        expected_revision: None,
                    }
                )
                .unwrap_err()
                .code,
            "workspace.invalid-scope"
        );
        assert_eq!(
            store
                .create_workspace(CreateWorkspaceInput {
                    name: "bad".into(),
                    path: "/".into()
                })
                .unwrap_err()
                .code,
            "workspace.invalid-path"
        );
        assert_eq!(
            store
                .create_workspace(CreateWorkspaceInput {
                    name: "missing".into(),
                    path: dir.path().join("missing").to_string_lossy().into_owned(),
                })
                .unwrap_err()
                .code,
            "workspace.path-missing"
        );
        let file_path = dir.path().join("not-a-directory.txt");
        fs::write(&file_path, "fixture").unwrap();
        assert_eq!(
            store
                .create_workspace(CreateWorkspaceInput {
                    name: "file".into(),
                    path: file_path.to_string_lossy().into_owned(),
                })
                .unwrap_err()
                .code,
            "workspace.invalid-path"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = dir.path().join("linked-source");
            symlink(&workspace.path, &link_path).unwrap();
            assert_eq!(
                store
                    .create_workspace(CreateWorkspaceInput {
                        name: "symlink".into(),
                        path: link_path.to_string_lossy().into_owned(),
                    })
                    .unwrap_err()
                    .code,
                "workspace.invalid-path"
            );
        }

        let agent = create_agent(&mut store, "Lead", "claude").unwrap();
        let team = store
            .create_team(CreateTeamInput {
                name: "Uses workspace".into(),
                supervisor_id: agent.id,
                workspace_id: workspace.id.clone(),
            })
            .unwrap();
        let other = create_workspace(&mut store, dir.path(), "other");
        assert_eq!(
            store
                .rename_workspace(&workspace.id, &other.name, Some(workspace.revision))
                .unwrap_err()
                .code,
            "workspace.duplicate"
        );
        fs::remove_dir(&workspace.path).unwrap();
        let renamed = store
            .rename_workspace(&workspace.id, "renamed source", Some(workspace.revision))
            .unwrap();
        assert_eq!(renamed.id, workspace.id);
        assert!(!renamed.path_available);
        assert_eq!(
            store.get_team(&team.id).unwrap().workspace,
            "renamed source"
        );
        assert_eq!(
            store
                .create_team(CreateTeamInput {
                    name: "Missing workspace team".into(),
                    supervisor_id: team.supervisor_id.clone(),
                    workspace_id: workspace.id.clone(),
                })
                .unwrap_err()
                .code,
            "team.workspace-unavailable"
        );
        assert_eq!(
            store
                .update_team(
                    &team.id,
                    UpdateTeamInput {
                        name: Some("Should not commit".into()),
                        expected_revision: Some(team.revision),
                        ..Default::default()
                    }
                )
                .unwrap_err()
                .code,
            "team.workspace-unavailable"
        );
        assert_eq!(store.get_team(&team.id).unwrap().name, team.name);
        assert_eq!(
            store
                .rename_workspace(&workspace.id, "stale", Some(workspace.revision))
                .unwrap_err()
                .code,
            "workspace.revision-conflict"
        );
        assert_eq!(
            store.archive_workspace(&workspace.id).unwrap_err().code,
            "workspace.in-use"
        );
        store.archive_workspace(&other.id).unwrap();
        assert_eq!(
            store.get_workspace(&other.id).unwrap_err().code,
            "workspace.not-found"
        );
    }

    #[test]
    fn team_graph_changes_are_transactional_and_reject_cycles_and_dangling_parents() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite3")).unwrap();
        let workspace = create_workspace(&mut store, dir.path(), "workspace");
        let a = create_agent(&mut store, "A", "claude").unwrap();
        let b = create_agent(&mut store, "B", "codex").unwrap();
        let c = create_agent(&mut store, "C", "opencode").unwrap();
        let team = store
            .create_team(CreateTeamInput {
                name: "Graph".into(),
                supervisor_id: a.id.clone(),
                workspace_id: workspace.id,
            })
            .unwrap();
        store
            .add_team_member(
                &team.id,
                AddTeamMemberInput {
                    agent_id: b.id.clone(),
                    reports_to_agent_id: a.id.clone(),
                },
            )
            .unwrap();
        store
            .add_team_member(
                &team.id,
                AddTeamMemberInput {
                    agent_id: c.id.clone(),
                    reports_to_agent_id: b.id.clone(),
                },
            )
            .unwrap();
        assert_eq!(
            store
                .add_team_member(
                    &team.id,
                    AddTeamMemberInput {
                        agent_id: c.id.clone(),
                        reports_to_agent_id: a.id.clone()
                    }
                )
                .unwrap_err()
                .code,
            "team.member-exists"
        );

        let before = store.get_team(&team.id).unwrap();
        let transferred = store
            .update_team(
                &team.id,
                UpdateTeamInput {
                    supervisor_id: Some(b.id.clone()),
                    expected_revision: Some(before.revision),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(transferred.supervisor_id, b.id);
        assert_eq!(transferred.members[0].agent_id, b.id);
        assert_eq!(transferred.members[0].reports_to_agent_id, None);
        assert_eq!(
            transferred.members[1].reports_to_agent_id.as_deref(),
            Some(b.id.as_str())
        );
        store.set_team_reports_to(&team.id, &c.id, &a.id).unwrap();
        let before = store.get_team(&team.id).unwrap();
        assert_eq!(
            store
                .update_team(
                    &team.id,
                    UpdateTeamInput {
                        name: Some("stale".into()),
                        expected_revision: Some(before.revision - 1),
                        ..Default::default()
                    }
                )
                .unwrap_err()
                .code,
            "team.revision-conflict"
        );
        assert_eq!(
            store
                .set_team_reports_to(&team.id, &a.id, &c.id)
                .unwrap_err()
                .code,
            "team.cycle"
        );
        assert_eq!(store.get_team(&team.id).unwrap(), before);
        assert_eq!(
            store.remove_team_member(&team.id, &a.id).unwrap_err().code,
            "team.supervisor-required"
        );
        assert_eq!(
            store
                .set_team_reports_to(&team.id, &c.id, "outside")
                .unwrap_err()
                .code,
            "team.member-not-found"
        );
        assert_eq!(store.get_team(&team.id).unwrap(), before);
    }

    #[test]
    fn entity_updates_use_optimistic_revisions() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite3")).unwrap();
        let agent = create_agent(&mut store, "Revision", "claude").unwrap();
        let updated = store
            .update_agent(
                &agent.id,
                UpdateAgentInput {
                    name: Some("Updated".into()),
                    expected_revision: Some(agent.revision),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.revision, agent.revision + 1);
        assert_eq!(
            store
                .update_agent(
                    &agent.id,
                    UpdateAgentInput {
                        name: Some("Stale".into()),
                        expected_revision: Some(agent.revision),
                        ..Default::default()
                    }
                )
                .unwrap_err()
                .code,
            "agent.revision-conflict"
        );
        assert_eq!(store.get_agent(&agent.id).unwrap().name, "Updated");
    }
}
