import type {
  Agent,
  AgentPermissions,
  AgentState,
  ApprovalDecision,
  ApprovalRequest,
  AppSettings,
  CreateAgentInput,
  CreateTaskInput,
  ExecutionReadiness,
  LocalTool,
  NotificationItem,
  RunArtifactGroup,
  RunRecord,
  Subtask,
  Task,
  TaskMessage,
  Team,
  TeamMember,
  UpdateAgentInput,
  Workspace,
} from "@/types";
import { ServiceError } from "@/types";
import type {
  AgentService,
  ExecutionService,
  ApprovalService,
  EventService,
  NotificationService,
  RunService,
  SettingsService,
  TaskService,
  TeamService,
  ToolService,
  WorkspaceService,
} from "@/services/contracts";
import type { Services } from "@/services/contracts";
import { MockDb, newId } from "./db";

/** 极简事件总线：仅内存，替代 Tauri 事件通道。 */
class Emitter<T> {
  private handlers = new Set<(value: T) => void>();
  subscribe(handler: (value: T) => void): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }
  emit(value: T): void {
    this.handlers.forEach((handler) => handler(value));
  }
}

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

function statusForTool(state: LocalTool["state"]): AgentState {
  if (state === "ready") return "ready";
  if (state === "auth-required") return "auth-required";
  return "not-ready";
}

/** 演示用延迟：明确不做“假装真实执行”的动画，只用于体现排队/安全边界等状态。 */
const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const unsupportedExecutionReadiness = (): ExecutionReadiness => ({
  ready: false,
  version: null,
  authState: "unknown",
  message: "真实只读执行仅在桌面端提供",
  reasonCode: "execution.not-supported",
  policyStatus: "unverified",
  checks: [
    { id: "localStorage", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "activeRun", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "processState", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "platform", status: "unsupported", reasonCode: "execution.not-supported" },
    { id: "cliDiscovery", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "cliVersion", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "authentication", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "managedPolicy", status: "unknown", reasonCode: "execution.not-supported" },
    { id: "taskStart", status: "blocked", reasonCode: "execution.not-supported" },
  ],
  blockers: [{ check: "taskStart", reasonCode: "execution.not-supported" }],
});

export type MockServices = Services;

export function createMockServices(): Services {
  const db = new MockDb();
  const taskBus = new Emitter<Task>();
  const approvalBus = new Emitter<ApprovalRequest>();
  const scanBus = new Emitter<{ phase: "scanning" | "done" | "failed"; message?: string }>();

  const findTask = (id: string): Task => {
    const task = db.tasks.find((item) => item.id === id);
    if (!task) throw new ServiceError("任务不存在或已被归档", "task.not-found", id);
    return task;
  };
  const findTeam = (id: string): Team => {
    const team = db.teams.find((item) => item.id === id);
    if (!team) throw new ServiceError("团队不存在", "team.not-found", id);
    return team;
  };
  const emitTask = (task: Task) => taskBus.emit(clone(task));

  const pushSystemMessage = (task: Task, text: string) => {
    const message: TaskMessage = { id: newId("m"), author: "supervisor", authorName: `${task.supervisor} · 主管`, time: "现在", text };
    task.messages = [...task.messages, message];
  };

  const agents: AgentService = {
    async listAgents() {
      return clone(db.agents);
    },
    async createAgent(input: CreateAgentInput) {
      if (!input.name.trim()) throw new ServiceError("请填写 Agent 名称", "agent.invalid-input");
      if (db.agents.some((agent) => agent.name === input.name)) {
        throw new ServiceError("该名称已存在", "agent.duplicate", input.name);
      }
      const tool = db.tools.find((item) => item.id === input.toolId);
      if (!tool) throw new ServiceError("执行工具不存在", "agent.invalid-input", input.toolId);
      const agent: Agent = {
        id: newId("r"),
        name: input.name.trim(),
        role: input.role || "成员",
        tool: `${tool.name} · ${tool.version}`,
        toolId: tool.id,
        state: statusForTool(tool.state),
        description: input.description ?? "",
        instructions: input.instructions ?? "",
        permissions: { read: true, write: false, shell: false, network: false, ...input.permissions },
      };
      db.agents = [...db.agents, agent];
      return clone(agent);
    },
    async updateAgent(id: string, input: UpdateAgentInput) {
      const agent = db.agents.find((item) => item.id === id);
      if (!agent) throw new ServiceError("角色不存在", "agent.not-found", id);
      if (input.name && input.name !== agent.name && db.agents.some((item) => item.name === input.name)) {
        throw new ServiceError("该名称已存在", "agent.duplicate", input.name);
      }
      const tool = input.toolId ? db.tools.find((item) => item.id === input.toolId) : undefined;
      Object.assign(agent, input, tool ? { tool: `${tool.name} · ${tool.version}`, state: statusForTool(tool.state) } : {});
      // 同步团队中的展示信息（名称/职责/状态），避免页面互相矛盾
      db.teams.forEach((team) =>
        team.members.forEach((member) => {
          if (member.agentId !== id) return;
          member.name = agent.name;
          member.role = agent.role;
          member.state = agent.state;
        }),
      );
      return clone(agent);
    },
    async deleteAgent(id: string) {
      const agent = db.agents.find((item) => item.id === id);
      if (!agent) throw new ServiceError("角色不存在", "agent.not-found", id);
      const isSupervisor = db.teams.some((team) => team.supervisorId === id);
      if (isSupervisor) throw new ServiceError("该角色是团队主管，请先指定新的主管", "agent.in-use", id);
      db.agents = db.agents.filter((item) => item.id !== id);
      db.teams.forEach((team) => {
        team.members = team.members.filter((member) => member.agentId !== id);
      });
    },
    async setPermissions(id: string, permissions: Partial<AgentPermissions>) {
      const agent = db.agents.find((item) => item.id === id);
      if (!agent) throw new ServiceError("角色不存在", "agent.not-found", id);
      agent.permissions = { ...agent.permissions, ...permissions };
      return clone(agent);
    },
  };

  const tools: ToolService = {
    async scanTools() {
      scanBus.emit({ phase: "scanning", message: "正在探测本机可执行工具" });
      await wait(1200);
      scanBus.emit({ phase: "done", message: "扫描完成：1 个可用、1 个待验证、2 个未安装/无法检测" });
      return clone(db.tools);
    },
    async verifyTool(id: string) {
      const tool = db.tools.find((item) => item.id === id);
      if (!tool) throw new ServiceError("工具不存在", "tool.not-found", id);
      await wait(900);
      if (tool.state === "auth-required") {
        tool.state = "ready";
        tool.note = "已验证登录 · 可用于任务执行";
      } else if (tool.state !== "ready") {
        // 明确：无法确认时不假定成功
        tool.note = "无法确认状态，仍显示为“无法检测”";
      }
      return clone(tool);
    },
    async setToolPath(id: string, path: string) {
      const tool = db.tools.find((item) => item.id === id);
      if (!tool) throw new ServiceError("工具不存在", "tool.not-found", id);
      if (!path.trim()) throw new ServiceError("路径不能为空", "tool.invalid-path");
      tool.path = path.trim();
      tool.state = "undetectable";
      tool.note = "已手动指定路径，等待验证（无法检测时不会假定可用）";
      return clone(tool);
    },
    async setDefaultTool(id: string) {
      if (!db.tools.some((item) => item.id === id)) throw new ServiceError("工具不存在", "tool.not-found", id);
      db.defaultToolId = id;
    },
    async getDefaultToolId() {
      return db.defaultToolId;
    },
  };

  const teams: TeamService = {
    async listTeams() {
      return clone(db.teams);
    },
    async getTeam(id: string) {
      return clone(findTeam(id));
    },
    async createTeam(input) {
      const team: Team = {
        id: newId("tm"),
        name: input.name,
        supervisorId: input.supervisorId,
        workspace: input.workspace,
        members: [
          {
            agentId: input.supervisorId,
            name: db.agents.find((agent) => agent.id === input.supervisorId)?.name ?? "主管",
            role: "主管 · 向用户汇报",
            reportsTo: "user",
            tool: "Claude Code",
            state: "ready",
          },
        ],
      };
      db.teams = [...db.teams, team];
      return clone(team);
    },
    async updateTeam(id, input) {
      const team = findTeam(id);
      Object.assign(team, input);
      return clone(team);
    },
    async addMember(teamId, input) {
      const team = findTeam(teamId);
      if (team.members.some((member) => member.agentId === input.agentId)) {
        throw new ServiceError("该成员已在团队中", "team.member-exists");
      }
      if (input.agentId === team.supervisorId) {
        throw new ServiceError("不能把主管添加为自己的下属", "team.self-management");
      }
      const agent = db.agents.find((item) => item.id === input.agentId);
      if (!agent) throw new ServiceError("角色不存在", "agent.not-found", input.agentId);
      const member: TeamMember = {
        agentId: agent.id,
        name: agent.name,
        role: agent.role,
        reportsTo: input.reportsTo,
        tool: agent.tool.split(" · ")[0] ?? agent.tool,
        state: agent.state,
      };
      team.members = [...team.members, member];
      return clone(team);
    },
    async removeMember(teamId, agentId) {
      const team = findTeam(teamId);
      if (team.supervisorId === agentId) {
        throw new ServiceError("主管不能直接移除，请先指定新的主管", "team.supervisor-required");
      }
      team.members = team.members.filter((member) => member.agentId !== agentId);
      team.members.forEach((member) => {
        if (member.reportsTo === agentId) member.reportsTo = team.supervisorId;
      });
      return clone(team);
    },
    async setReportsTo(teamId, agentId, reportsTo) {
      const team = findTeam(teamId);
      const member = team.members.find((item) => item.agentId === agentId);
      if (!member) throw new ServiceError("成员不存在", "team.member-not-found", agentId);
      if (reportsTo === agentId) throw new ServiceError("不能把自己设为上级", "team.self-management");
      // 环检测：沿上级链向上走，若遇到自己则成环
      const chain: string[] = [agentId];
      let cursor = reportsTo;
      let guard = 0;
      while (cursor && cursor !== "user" && guard++ < 50) {
        if (cursor === agentId) {
          throw new ServiceError("会产生间接环", "team.cycle", [...chain, cursor].join(" → "));
        }
        chain.push(cursor);
        cursor = team.members.find((item) => item.agentId === cursor)?.reportsTo ?? "user";
      }
      member.reportsTo = reportsTo;
      if (reportsTo === "user") team.supervisorId = agentId;
      return clone(team);
    },
    async listMembers(teamId) {
      return clone(findTeam(teamId).members);
    },
  };

  const tasks: TaskService = {
    async listTasks() {
      return clone(db.tasks);
    },
    async getTask(id) {
      return clone(findTask(id));
    },
    async createTask(input: CreateTaskInput) {
      if (!input.description.trim()) throw new ServiceError("请填写任务描述", "task.invalid-input");
      let executorLabel: string;
      let supervisorName: string;
      if (input.mode === "team") {
        if (input.agentId !== undefined) throw new ServiceError("团队模式不能同时指定单 Agent", "task.invalid-input");
        const team = input.teamId ? db.teams.find((item) => item.id === input.teamId) : undefined;
        if (!team) throw new ServiceError("请选择有效团队", "task.no-executor", input.teamId ?? "");
        const supervisor = db.agents.find((agent) => agent.id === team.supervisorId);
        if (!supervisor || supervisor.state !== "ready") {
          throw new ServiceError("所选团队的主管 Agent 当前不可用", "task.no-executor", team.supervisorId);
        }
        executorLabel = `${team.name} · ${team.members.length} 人`;
        supervisorName = supervisor.name;
      } else if (input.mode === "single-agent") {
        if (input.teamId !== undefined) throw new ServiceError("单 Agent 模式不能同时指定团队", "task.invalid-input");
        const agent = input.agentId ? db.agents.find((item) => item.id === input.agentId) : undefined;
        if (!agent || agent.state !== "ready") {
          throw new ServiceError("请选择可用的 Agent", "task.no-executor", input.agentId ?? "");
        }
        executorLabel = `${agent.name} · 单 Agent`;
        supervisorName = "无（单 Agent）";
      } else if (input.mode === "custom") {
        if (input.teamId !== undefined || input.agentId !== undefined) throw new ServiceError("自定义占位不能绑定团队或 Agent", "task.invalid-input");
        executorLabel = "自定义执行 · 计划待定义";
        supervisorName = "未分配";
      } else {
        throw new ServiceError("请选择有效的执行模式", "task.invalid-input");
      }
      const task: Task = {
        id: newId("t"),
        name: input.description.trim().slice(0, 42),
        workspace: input.workspace,
        mode: input.mode,
        team: executorLabel,
        runId: db.nextRunId(),
        supervisor: supervisorName,
        createdAt: new Date().toISOString(),
        startedAt: "未开始",
        state: input.mode === "custom" ? "draft" : "queued",
        stateNote: input.mode === "custom" ? "待配置占位 · 尚未定义执行计划" : "已创建 · 等待运行环境检查",
        subtasks: [],
        messages: input.attachments?.length
          ? [{ id: newId("m"), author: "user", authorName: "你", time: "现在", text: `已附加 ${input.attachments.length} 个文件（模拟）：${input.attachments.join("、")}` }]
          : [],
        instruction: null,
      };
      db.tasks = [task, ...db.tasks];
      emitTask(task);
      if (input.mode !== "custom") {
        // 演示：1.4s 后模拟环境检查并进入执行中（真实实现由 Rust 派发确认）
        setTimeout(() => {
          const current = db.tasks.find((item) => item.id === task.id);
          if (!current || current.state !== "queued") return;
          current.state = "running";
          current.startedAt = "现在";
          current.stateNote = "Mock 演示状态：运行环境检查通过 · 已开始执行";
          emitTask(current);
        }, 1400);
      }
      return clone(task);
    },
    async pauseTask(id) {
      const task = findTask(id);
      if (!["running", "waiting-approval"].includes(task.state)) {
        throw new ServiceError("当前状态不支持暂停", "task.illegal-transition", task.state);
      }
      task.state = "pause-pending";
      task.stateNote = "等待暂停：已停止派发新工作";
      emitTask(task);
      setTimeout(() => {
        const current = db.tasks.find((item) => item.id === id);
        if (!current || current.state !== "pause-pending") return;
        current.state = "paused";
        current.stateNote = "已暂停 · 全部执行到达安全边界";
        emitTask(current);
      }, 1700);
      return clone(task);
    },
    async resumeTask(id) {
      const task = findTask(id);
      if (task.state !== "paused") throw new ServiceError("当前状态不支持继续", "task.illegal-transition", task.state);
      task.state = "running";
      task.stateNote = "继续 · 已通过权限与依赖检查";
      emitTask(task);
      return clone(task);
    },
    async stopTask(id) {
      const task = findTask(id);
      task.state = "stop-pending";
      task.stateNote = "正在请求停止 · 等待运行时确认";
      emitTask(task);
      setTimeout(() => {
        const current = db.tasks.find((item) => item.id === id);
        if (!current || current.state !== "stop-pending") return;
        current.state = "stopped";
        current.stateNote = "已停止 · 2 个运行时已确认";
        emitTask(current);
      }, 1800);
      return clone(task);
    },
    async retryFailed(id) {
      const task = findTask(id);
      const previousRun = task.runId;
      task.runId = db.nextRunId();
      task.state = "running";
      task.stateNote = `新 Run 已启动 · 旧 Run ${previousRun} 记录保留`;
      task.subtasks = task.subtasks.map((subtask) =>
        ["failure", "blocked", "skipped"].includes(subtask.state)
          ? { ...subtask, state: "running" as const, result: "重试中（新 Run）" }
          : subtask,
      );
      emitTask(task);
      return clone(task);
    },
    async skipAndFinalizePartial(id) {
      const task = findTask(id);
      task.state = "partial";
      task.stateNote = "部分完成 · 已标注跳过范围";
      task.subtasks = task.subtasks.map((subtask) =>
        ["failure", "blocked", "waiting-approval"].includes(subtask.state)
          ? { ...subtask, state: "skipped" as const, result: "已跳过 · 结果标注为不完整" }
          : subtask,
      );
      emitTask(task);
      return clone(task);
    },
    async listSubtasks(taskId): Promise<Subtask[]> {
      return clone(findTask(taskId).subtasks);
    },
    subscribeTaskState(taskId, handler) {
      return taskBus.subscribe((task) => {
        if (task.id === taskId) handler(task.state, task);
      });
    },
  };

  const approvals: ApprovalService = {
    async listApprovals() {
      return clone(db.approvals);
    },
    async getApproval(id) {
      const approval = db.approvals.find((item) => item.id === id);
      if (!approval) throw new ServiceError("审批记录不存在", "approval.not-found", id);
      return clone(approval);
    },
    async decide(id: string, decision: ApprovalDecision) {
      const approval = db.approvals.find((item) => item.id === id);
      if (!approval) throw new ServiceError("审批记录不存在", "approval.not-found", id);
      if (approval.state !== "pending") throw new ServiceError("该请求已有最终记录", "approval.already-decided", approval.state);

      if (decision === "allow-once") {
        approval.state = "allowed-once";
        approval.history = [...approval.history, { time: "现在", title: "已允许一次（你）", detail: "作用域：仅本文件 · 仅本次 · 仅本 Run" }];
        const task = db.tasks.find((item) => item.id === approval.taskId);
        if (task) {
          task.state = "running";
          task.stateNote = "已获得一次性授权，继续执行";
          task.subtasks = task.subtasks.map((subtask) =>
            subtask.state === "waiting-approval" ? { ...subtask, state: "running" as const, result: "已获授权，正在写入" } : subtask,
          );
          pushSystemMessage(task, "已收到你的“仅此一次允许”。我继续第 4 步写入，完成后会汇总结论。");
          emitTask(task);
        }
      } else {
        approval.state = "denied";
        approval.history = [...approval.history, { time: "现在", title: "已拒绝（你）", detail: "拒绝后不会重试；需要重新发起合规申请" }];
        const task = db.tasks.find((item) => item.id === approval.taskId);
        if (task) {
          task.state = "needs-attention";
          task.stateNote = "审批被拒绝 · 相关步骤未执行";
          task.subtasks = task.subtasks.map((subtask) =>
            subtask.state === "waiting-approval" ? { ...subtask, state: "blocked" as const, result: "授权被拒绝，未执行" } : subtask,
          );
          pushSystemMessage(task, "已收到拒绝。该步骤不会执行，我也不会绕过你的决定重试。你可以选择跳过该步骤生成部分成果，或停止任务。");
          emitTask(task);
        }
      }
      approvalBus.emit(clone(approval));
      return clone(approval);
    },
    async canGrantTaskScope() {
      // 未来需要可靠的授权范围与撤销能力；当前明确返回 false
      return false;
    },
    subscribe(handler) {
      return approvalBus.subscribe((approval) => handler(approval, approval.state));
    },
  };

  const runs: RunService = {
    async listRuns(filter) {
      const extra: RunRecord[] = Array.from({ length: db.extraRuns }, (_, index) => ({
        id: `#12${30 - index}`,
        taskName: `更早的演示 Run · 归档批次 ${index + 1}`,
        executor: "数据小组",
        state: "finished",
        startedAt: "上周",
        duration: "5m",
      }));
      let items = [...db.runs, ...extra];
      if (filter?.result && filter.result !== "全部") {
        const map: Record<string, RunRecord["state"]> = { 已完成: "finished", 失败: "crashed", 部分完成: "finished", 已停止: "stop-requested", 状态未知: "unknown" };
        items = items.filter((item) => item.state === map[filter.result as string]);
      }
      if (filter?.type === "仅失败") items = items.filter((item) => item.state === "crashed");
      return { items: clone(items), hasMore: db.extraRuns < 24 };
    },
    async getPartialArtifacts(): Promise<RunArtifactGroup[]> {
      return clone(db.partialGroups);
    },
    async listArtifacts() {
      return [
        { path: "docs/compare-draft.md", state: "running" as const, note: "进行中 · 2.4 KB" },
        { path: "exports/sources.csv", state: "done" as const, note: "已完成 · 12 行" },
        { path: "docs/report-q3.md", state: "waiting-approval" as const, note: "等待授权写入" },
      ];
    },
    async exportRunsCsv() {
      const rows = ["run,task,executor,state,start,duration"];
      db.runs.forEach((run) => rows.push(`${run.id},"${run.taskName}",${run.executor},${run.state},${run.startedAt},${run.duration}`));
      return rows.join("\n");
    },
  };

  const workspaces: WorkspaceService = {
    async listWorkspaces() {
      return clone(db.workspaces);
    },
    async createWorkspace(input) {
      if (db.workspaces.some((item) => item.name === input.name)) {
        throw new ServiceError("同名工作空间已存在", "workspace.duplicate", input.name);
      }
      const workspace: Workspace = {
        id: newId("w"),
        name: input.name,
        path: input.path,
        taskCount: 0,
        state: "ready",
        read: ["docs/"],
        write: ["docs/"],
        deny: ["~/.ssh", "~/.config"],
      };
      db.workspaces = [...db.workspaces, workspace];
      return clone(workspace);
    },
    async renameWorkspace(id, name) {
      const workspace = db.workspaces.find((item) => item.id === id);
      if (!workspace) throw new ServiceError("工作空间不存在", "workspace.not-found", id);
      const normalized = name.trim().toLocaleLowerCase();
      if (!normalized) throw new ServiceError("工作空间名称不能为空", "workspace.invalid-input");
      if (db.workspaces.some((item) => item.id !== id && item.name.trim().toLocaleLowerCase() === normalized)) {
        throw new ServiceError("同名工作空间已存在", "workspace.duplicate", name);
      }
      const oldName = workspace.name;
      workspace.name = name.trim();
      db.teams.forEach((team) => { if (team.workspace === oldName) team.workspace = workspace.name; });
      return clone(workspace);
    },
    async updateBoundaries(id, input) {
      const workspace = db.workspaces.find((item) => item.id === id);
      if (!workspace) throw new ServiceError("工作空间不存在", "workspace.not-found", id);
      workspace.read = input.read;
      workspace.write = input.write;
      return clone(workspace);
    },
    async archiveWorkspace(id) {
      const workspace = db.workspaces.find((item) => item.id === id);
      if (!workspace) throw new ServiceError("工作空间不存在", "workspace.not-found", id);
      if (db.teams.some((team) => team.workspace === workspace.name)) throw new ServiceError("该工作空间仍被团队引用", "workspace.in-use");
      db.workspaces = db.workspaces.filter((item) => item.id !== id);
    },
    async resolveConflict(id) {
      const workspace = db.workspaces.find((item) => item.id === id);
      if (!workspace) throw new ServiceError("工作空间不存在", "workspace.not-found", id);
      if (workspace.state !== "conflict") throw new ServiceError("当前没有冲突需要处理", "workspace.no-conflict");
      workspace.state = "ready";
      workspace.conflictInfo = "已排队其中之一（演示）";
      return clone(workspace);
    },
    async resolveOpenPath(id) {
      const workspace = db.workspaces.find((item) => item.id === id);
      if (!workspace) throw new ServiceError("工作空间不存在", "workspace.not-found", id);
      return workspace.path;
    },
  };

  const settings: SettingsService = {
    async getSettings() {
      return clone(db.settings) as AppSettings;
    },
    async updateSettings(patch) {
      db.settings = { ...db.settings, ...patch };
      return clone(db.settings) as AppSettings;
    },
    async resetAppearance() {
      db.settings = { ...db.settings, theme: "system", density: "comfortable", grayscalePreview: false, reducedMotion: false, platform: "mac" };
      return clone(db.settings) as AppSettings;
    },
    async resetDemoData() {
      // 仅重置 Mock 数据；正式实现应改为清理本地缓存并要求二次确认
      const fresh = new MockDb();
      Object.assign(db, fresh);
    },
  };

  const notifications: NotificationService = {
    async listNotifications() {
      return clone(db.notifications);
    },
    async markRead(id) {
      const item = db.notifications.find((note) => note.id === id);
      if (item) item.unread = false;
    },
    async markAllRead() {
      db.notifications = db.notifications.map((note) => ({ ...note, unread: false }));
    },
    async dismiss(id) {
      db.notifications = db.notifications.filter((note) => note.id !== id);
    },
    async unreadCount() {
      return db.notifications.filter((note) => note.unread).length;
    },
  };

  const events: EventService = {
    subscribeTaskLogs(_taskId, handler) {
      // 回放历史日志，然后每 3s 追加一条心跳（明确标注为演示）
      db.logs.forEach((line) => handler(line));
      const timer = window.setInterval(() => {
        handler({ time: "现在", level: "DEBUG", actor: "adapter", message: "演示心跳 · 无真实执行" });
      }, 3000);
      return () => window.clearInterval(timer);
    },
    subscribeTaskState(handler) {
      return taskBus.subscribe(handler);
    },
    subscribeApprovals(handler) {
      return approvalBus.subscribe(handler);
    },
    subscribeToolScan(handler) {
      return scanBus.subscribe(handler);
    },
  };

  const execution: ExecutionService = {
    async selectWorkspaceDirectory() { return null; },
    async readiness() { return unsupportedExecutionReadiness(); },
    async listUnknownRuns() { return []; },
    async inspectUnknownRun() { throw new ServiceError("恢复检查仅在桌面端提供", "execution.not-supported"); },
    async resolveUnknownRun() { throw new ServiceError("恢复归档仅在桌面端提供", "execution.not-supported"); },
    async startReadOnlyTask() { throw new ServiceError("真实只读执行仅在桌面端提供", "execution.not-supported"); },
    async stopRun() { throw new ServiceError("真实只读执行仅在桌面端提供", "execution.not-supported"); },
    async getRun() { throw new ServiceError("真实只读执行仅在桌面端提供", "execution.not-supported"); },
    async getTask() { throw new ServiceError("真实只读执行仅在桌面端提供", "execution.not-supported"); },
    async listTasks() { return []; },
    async listRuns() { return { items: [], hasMore: false, nextCursor: null }; },
    async listRunEvents() { return []; },
    subscribeRunState() { return () => undefined; },
    subscribeRunEvents() { return () => undefined; },
  };

  return { agents, tools, teams, tasks, approvals, runs, workspaces, settings, notifications, events, execution };
}

export type { NotificationItem, ApprovalRequest };
