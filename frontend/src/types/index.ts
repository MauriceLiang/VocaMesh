/**
 * VocaMesh 领域类型定义。
 *
 * 用途：前端所有页面、Store、Service 共享的类型契约。
 * 设计来源：UI/prototype/data.js 与 UI/handoff/UX_STATE_MATRIX.md。
 * 注意：这些类型描述“界面需要的形状”，不描述 Rust Core 的内部模型；
 *      对接时由 services/adapters 做映射。
 */

/** 语义状态族：决定徽标的颜色/图标，不允许页面自行判定颜色。 */
export type StatusTone = "success" | "failure" | "blocked" | "neutral" | "unknown";

/** 任务状态（面向用户的展示态）。 */
export type TaskState =
  | "draft"
  | "queued"
  | "running"
  | "waiting-approval"
  | "pause-pending"
  | "paused"
  | "stop-pending"
  | "stopped"
  | "needs-attention"
  | "partial"
  | "done"
  | "failed"
  | "unknown";

/** 子任务状态。 */
export type SubtaskState =
  | "pending"
  | "waiting-dependency"
  | "ready"
  | "running"
  | "waiting-approval"
  | "success"
  | "failure"
  | "blocked"
  | "skipped"
  | "cancelled"
  | "unknown";

/** 本地执行工具（CLI / 适配器）状态。 */
export type ToolState = "ready" | "pending" | "auth-required" | "missing" | "incompatible" | "undetectable";
export type ToolAuthState = "authenticated" | "unauthenticated" | "unknown" | "unsupported";
export type ToolSource = "path" | "standard-location" | "manual";

/** Run（执行尝试）状态。不可覆盖：重试产生新 Run。 */
export type RunState = "pending" | "starting" | "running" | "waiting-input" | "stop-requested" | "finished" | "crashed" | "unknown";

/** 审批状态。 */
export type ApprovalState = "pending" | "allowed-once" | "allowed-task" | "denied" | "revoked" | "expired" | "unknown";

/** 定向指令投递状态。 */
export type InstructionState = "queued" | "delivered" | "running" | "applied" | "not-applied" | "waiting-point" | "failed";

/** Agent 角色状态。 */
export type AgentState = "ready" | "auth-required" | "not-ready" | "unknown";

export interface Agent {
  id: string;
  name: string;
  /** 职责描述（面向用户的一句话）。 */
  role: string;
  /** 执行工具展示名，例如 "Claude Code · v2.1.4"。 */
  tool: string;
  toolId: string;
  state: AgentState;
  description: string;
  /** 角色指令（高级配置）。 */
  instructions: string;
  permissions: AgentPermissions;
}

export interface AgentPermissions {
  read: boolean;
  write: boolean;
  shell: boolean;
  network: boolean;
}

export interface CreateAgentInput {
  name: string;
  role: string;
  toolId: string;
  description?: string;
  instructions?: string;
  permissions?: Partial<AgentPermissions>;
}

export interface UpdateAgentInput {
  name?: string;
  role?: string;
  toolId?: string;
  description?: string;
  instructions?: string;
  permissions?: Partial<AgentPermissions>;
}

export interface LocalTool {
  id: string;
  name: string;
  adapter: string;
  version: string;
  /** 可执行文件路径；未检测到时为 "未检测到"。 */
  path: string;
  state: ToolState;
  /** 面向用户的解释（为什么是这个状态、下一步做什么）。 */
  note: string;
  /** 认证和安装状态分开；未知或不支持不会映射为 ready。 */
  authState?: ToolAuthState;
  /** Unix epoch milliseconds. */
  detectedAt?: number;
  source?: ToolSource;
}

export interface TeamMember {
  /** 成员引用 Agent 角色的名称（原型以名称建键，工程内应迁移到 agentId）。 */
  agentId: string;
  name: string;
  role: string;
  /** 直属上级的 agentId 或 "user"（表示直接向用户汇报）。 */
  reportsTo: string;
  tool: string;
  state: AgentState;
}

export interface Team {
  id: string;
  name: string;
  supervisorId: string;
  workspace: string;
  members: TeamMember[];
}

export interface Subtask {
  id: string;
  name: string;
  /** 执行者展示名，例如 "分析师 · Codex CLI"。 */
  assignee: string;
  state: SubtaskState;
  /** 结果摘要或阻塞原因。 */
  result: string;
}

export interface TaskMessage {
  id: string;
  author: "user" | "supervisor" | "system";
  authorName: string;
  time: string;
  text: string;
}

export interface TaskInstruction {
  target: string;
  text: string;
  state: InstructionState;
  impact?: string;
}

export interface Task {
  id: string;
  name: string;
  workspace: string;
  team: string;
  runId: string;
  supervisor: string;
  startedAt: string;
  state: TaskState;
  stateNote: string;
  subtasks: Subtask[];
  messages: TaskMessage[];
  instruction?: TaskInstruction | null;
}

export interface CreateTaskInput {
  description: string;
  workspace: string;
  mode: "single-agent" | "team" | "custom";
  teamId?: string;
  agentId?: string;
  attachments?: string[];
}

export interface ApprovalRequest {
  id: string;
  taskId: string;
  state: ApprovalState;
  age: string;
  title: string;
  agent: string;
  /** 请求的具体操作（命令 / 路径）。 */
  target: string;
  /** 差异/影响摘要。 */
  diff: string;
  /** 授权作用域（单文件/单次/任务内）。 */
  scope: string;
  risk: string[];
  history: ApprovalHistoryEntry[];
}

export interface ApprovalHistoryEntry {
  time: string;
  title: string;
  detail: string;
}

export type ApprovalDecision = "allow-once" | "deny";

export interface RunRecord {
  id: string;
  taskName: string;
  executor: string;
  state: RunState;
  startedAt: string;
  duration: string;
}

export interface RunArtifactGroup {
  kind: Exclude<StatusTone, "unknown">;
  label: string;
  items: Array<{ path: string; note: string }>;
}

export interface Workspace {
  id: string;
  name: string;
  path: string;
  taskCount: number;
  state: "ready" | "conflict" | "readonly";
  read: string[];
  write: string[];
  deny: string[];
  conflictInfo?: string;
}

export interface NotificationItem {
  id: string;
  kind: "blocked" | "failure" | "success" | "neutral";
  title: string;
  body: string;
  time: string;
  unread: boolean;
  target: { screen: string; taskId?: string; approvalId?: string };
}

export interface LogLine {
  time: string;
  level: "INFO" | "WARN" | "ERROR" | "DEBUG";
  actor: string;
  message: string;
}

export interface ActivityEntry {
  id: string;
  time: string;
  title: string;
  detail: string;
  tone: StatusTone;
}

export interface ResultArtifact {
  path: string;
  state: "running" | "done" | "waiting-approval";
  note: string;
}

export interface AppSettings {
  theme: "light" | "dark" | "system";
  density: "comfortable" | "compact";
  grayscalePreview: boolean;
  reducedMotion: boolean;
  platform: "mac" | "windows";
  closeBehavior: "window" | "app";
  concurrency: number;
  autoRetry: boolean;
  logRetention: "7 天" | "30 天" | "90 天" | "永久保留";
  notificationPrefs: { approval: boolean; failure: boolean; done: boolean; systemOnly: boolean };
}

/** 统一错误形状：Service 抛出的可展示错误。 */
export class ServiceError extends Error {
  constructor(
    message: string,
    readonly code: string,
    readonly detail?: string,
  ) {
    super(message);
    this.name = "ServiceError";
  }
}
