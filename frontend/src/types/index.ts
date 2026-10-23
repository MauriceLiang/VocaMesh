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
  | "starting"
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

export type TaskFilterState = "all" | TaskState;
export type TaskTimeFilter = "7d" | "30d" | "all";
export type TaskMode = "single-agent" | "team" | "custom";

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
export type ToolDiscoveryStatus = "missing" | "found" | "path-invalid" | "probe-failed";
export type ToolVersionStatus = "known" | "unknown" | "incompatible";
export type ToolIntegrationStatus = "discovery-only" | "adapter-available" | "unsupported-platform" | "blocked-by-policy";

export interface ProviderCapabilities {
  providerId: string;
  displayName: string;
  executableName: string;
  adapter: string;
  discoverySupported: boolean;
  versionProbeSupported: boolean;
  executionAdapterAvailable: boolean;
  authenticationProbeSupported: boolean;
  minimumVersion: string | null;
  unsupportedReasonCode: string | null;
}

/** Run（执行尝试）状态。不可覆盖：重试产生新 Run。 */
export type RunState = "pending" | "starting" | "running" | "waiting-input" | "stop-requested" | "stopped" | "finished" | "crashed" | "unknown";

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
  /** Native configured roles remain blocked while the runtime policy is unverified. */
  runtimeReason?: string;
  revision?: number;
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
  expectedRevision?: number;
}

export interface LocalTool {
  id: string;
  name: string;
  adapter: string;
  version: string;
  /** 可执行文件路径；未检测到时为 "未检测到"。 */
  path: string;
  /** 规范化后的目标路径；展示给用户的 path 保留所选 symlink。 */
  canonicalPath: string | null;
  state: ToolState;
  /** 面向用户的解释（为什么是这个状态、下一步做什么）。 */
  note: string;
  /** 认证和安装状态分开；未知或不支持不会映射为 ready。 */
  authState: ToolAuthState;
  discoveryStatus: ToolDiscoveryStatus;
  versionStatus: ToolVersionStatus;
  integrationStatus: ToolIntegrationStatus;
  reasonCode: string | null;
  runtimeReady: boolean;
  runtimeReasonCode: string | null;
  capabilities: ProviderCapabilities;
  /** Unix epoch milliseconds. */
  lastCheckedAt: number;
  source: ToolSource | null;
}

export interface TeamMember {
  /** 成员引用 Agent 角色的名称（原型以名称建键，工程内应迁移到 agentId）。 */
  agentId: string;
  name: string;
  role: string;
  /** 直属上级的 agentId 或 "user"（表示直接向用户汇报）。 */
  reportsTo: string;
  tool: string;
  providerId?: string;
  state: AgentState;
}

export interface Team {
  id: string;
  name: string;
  supervisorId: string;
  workspace: string;
  /** Stable native catalog reference; browser Mock continues to use display names. */
  workspaceId?: string;
  revision?: number;
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
  mode: TaskMode;
  team: string;
  runId: string;
  supervisor: string;
  /** 机器可读的任务创建时间；不代表最后更新时间或开始时间。 */
  createdAt: string;
  startedAt: string;
  state: TaskState;
  stateNote: string;
  subtasks: Subtask[];
  messages: TaskMessage[];
  instruction?: TaskInstruction | null;
  /** Present only for native read-only Claude Code tasks. */
  nativeExecution?: boolean;
  prompt?: string;
}

interface CreateTaskBase {
  description: string;
  workspace: string;
  attachments?: string[];
}

export type CreateTaskInput =
  | (CreateTaskBase & { mode: "single-agent"; agentId: string; teamId?: never })
  | (CreateTaskBase & { mode: "team"; teamId: string; agentId?: never })
  | (CreateTaskBase & { mode: "custom"; teamId?: never; agentId?: never });

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
  taskId?: string;
  providerId?: string;
  outcome?: string | null;
  summary?: string | null;
  outputTruncated?: boolean;
  finishedAt?: string | null;
  errorCode?: string | null;
}

export interface NativeTaskRecord {
  id: string;
  title: string;
  prompt: string;
  providerId: string;
  workspacePath: string;
  state: string;
  createdAt: number;
  updatedAt: number;
  currentRunId: string;
  authorizationScope: string;
  consentAt: number;
}

export interface NativeRunRecord {
  id: string;
  requestId: string;
  taskId: string;
  providerId: string;
  state: RunState;
  outcome: string | null;
  startedAt: number | null;
  finishedAt: number | null;
  exitCode: number | null;
  summary: string | null;
  lastSequence: number;
  outputTruncated: boolean;
  processId: number | null;
  errorCode: string | null;
  recoveryStatus: RecoveryStatus;
  recoveryCheckedAt: number | null;
}

export type RecoveryStatus = "none" | "required" | "resolved" | "manual-required";
export type ProcessGroupStatus = "exited-candidate" | "still-running" | "cannot-verify" | "manual-required";
export type SnapshotStatus = "exists" | "missing" | "cannot-verify";

export interface UnknownRunSummary {
  run: NativeRunRecord;
  taskTitle: string;
  snapshotStatus: SnapshotStatus;
}

export interface RecoveryInspection {
  run: NativeRunRecord;
  taskTitle: string;
  processGroupStatus: ProcessGroupStatus;
  snapshotStatus: SnapshotStatus;
}

export interface RecoveryResolution {
  run: NativeRunRecord;
  snapshotStatus: SnapshotStatus;
}

export interface NativeRunEvent {
  runId: string;
  sequence: number;
  timestamp: number;
  kind: string;
  text: string;
}

export interface ExecutionReadiness {
  ready: boolean;
  version: string | null;
  authState: ToolAuthState;
  message: string;
  reasonCode: string | null;
  policyStatus: ExecutionPolicyStatus;
  checks: ExecutionCheck[];
  blockers: ExecutionBlocker[];
}

export type ExecutionPolicyStatus = "verified" | "unverified";
export type ExecutionCheckId =
  | "localStorage"
  | "activeRun"
  | "processState"
  | "platform"
  | "cliDiscovery"
  | "cliVersion"
  | "authentication"
  | "managedPolicy"
  | "taskStart";
export type ExecutionCheckStatus = "passed" | "failed" | "blocked" | "unknown" | "unsupported";

export interface ExecutionCheck {
  id: ExecutionCheckId;
  status: ExecutionCheckStatus;
  reasonCode: string | null;
}

export interface ExecutionBlocker {
  check: ExecutionCheckId;
  reasonCode: string;
}

export interface StartReadOnlyTaskInput {
  requestId: string;
  title: string;
  prompt: string;
  providerId: "claude" | "codex";
  workspacePath: string;
  consent: boolean;
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
  /** False when the registered directory is missing, inaccessible, or no longer safe. */
  pathAvailable?: boolean;
  taskCount: number;
  state: "ready" | "conflict" | "readonly";
  read: string[];
  write: string[];
  deny: string[];
  revision?: number;
  /** Registration is not an execution or filesystem permission grant. */
  executionEnabled?: boolean;
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
  eventKind?: string;
  sequence?: number;
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
