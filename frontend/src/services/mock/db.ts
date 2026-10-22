import type {
  Agent,
  AppSettings,
  ApprovalRequest,
  LocalTool,
  LogLine,
  NotificationItem,
  RunRecord,
  Task,
  Team,
  Workspace,
} from "@/types";

/**
 * Mock 数据源（唯一事实来源）。
 *
 * 说明：
 * - 所有页面/Store 都通过 Service 读取这里的数据，避免不同页面出现互相矛盾的数字。
 * - 数据形状与 UI/prototype/data.js 一一对应，便于视觉对照。
 * - 这里不访问网络、不读写文件系统、不启动任何进程。
 */

const uid = (prefix: string) => `${prefix}_${Math.random().toString(36).slice(2, 8)}`;
const taskCreatedAt = (daysAgo: number) => new Date(Date.now() - daysAgo * 24 * 60 * 60 * 1000).toISOString();

export const seedAgents = (): Agent[] => [
  {
    id: "r1",
    name: "研究协调员",
    role: "主管 · 向用户汇报",
    tool: "Claude Code · v2.1.4",
    toolId: "claude",
    state: "ready",
    description:
      "作为主管：拆解任务、分配子任务、汇总结论并向我汇报。遇到需要授权或可能影响计划的变更时，先停下询问。",
    instructions:
      "1. 先确认目标与验收标准，再拆分阶段。\n2. 数据必须可追溯：每个结论附来源编号，缺数据留空。\n3. 不代替用户批准高风险操作；不在未确认时重试已失败的执行。",
    permissions: { read: true, write: true, shell: false, network: true },
  },
  {
    id: "r2",
    name: "研究员",
    role: "资料收集",
    tool: "Claude Code · v2.1.4",
    toolId: "claude",
    state: "ready",
    description: "收集公开资料并记录来源编号，不推测缺失数据。",
    instructions: "为每个结论附来源编号；无法确认的信息标注为“未验证”。",
    permissions: { read: true, write: false, shell: false, network: true },
  },
  {
    id: "r3",
    name: "分析师",
    role: "表格与结构",
    tool: "Codex CLI · v0.9.2",
    toolId: "codex",
    state: "auth-required",
    description: "生成对比表格与结构化草稿。",
    instructions: "输出 Markdown 表格；缺失单元格留空并在备注列说明。",
    permissions: { read: true, write: true, shell: false, network: false },
  },
  {
    id: "r4",
    name: "写作者",
    role: "结论与导出",
    tool: "Claude Code · v2.1.4",
    toolId: "claude",
    state: "not-ready",
    description: "撰写结论、整理措辞并导出。",
    instructions: "结论必须引用表格中的行号；不使用未经确认的数字。",
    permissions: { read: true, write: true, shell: false, network: false },
  },
  {
    id: "r5",
    name: "法务助手",
    role: "合同条款提取",
    tool: "本地模型 · 实验",
    toolId: "omni",
    state: "unknown",
    description: "读取合同并提取条款，不提供法律意见。",
    instructions: "只做提取与对比，不给出法律结论。",
    permissions: { read: true, write: false, shell: false, network: false },
  },
];

export const seedTools = (): LocalTool[] => [
  { id: "claude", name: "Claude Code", adapter: "claude-code", version: "v2.1.4", path: "/usr/local/bin/claude", state: "ready", note: "已验证登录 · 3 个角色在用" },
  { id: "codex", name: "Codex CLI", adapter: "codex-cli", version: "v0.9.2", path: "/opt/homebrew/bin/codex", state: "auth-required", note: "检测到配置文件，但无法确认登录状态" },
  { id: "opencode", name: "OpenCode", adapter: "opencode", version: "—", path: "未检测到", state: "missing", note: "需要手动安装后才能被角色使用" },
  { id: "omni", name: "Omni Adapter", adapter: "omni", version: "v1.2.0", path: "~/.local/bin/omni", state: "undetectable", note: "版本读取超时，可能不兼容当前系统" },
];

export const seedTeams = (): Team[] => [
  {
    id: "tm1",
    name: "调研团队",
    supervisorId: "r1",
    workspace: "research-2025",
    members: [
      { agentId: "r1", name: "研究协调员", role: "主管 · 向用户汇报", reportsTo: "user", tool: "Claude Code", state: "ready" },
      { agentId: "r2", name: "研究员", role: "资料收集", reportsTo: "r1", tool: "Claude Code", state: "ready" },
      { agentId: "r3", name: "分析师", role: "表格与结构", reportsTo: "r1", tool: "Codex CLI", state: "auth-required" },
      { agentId: "r4", name: "写作者", role: "结论与导出", reportsTo: "r1", tool: "Claude Code", state: "not-ready" },
    ],
  },
  {
    id: "tm2",
    name: "内容团队",
    supervisorId: "r1",
    workspace: "release-2.4",
    members: [
      { agentId: "r1", name: "研究协调员", role: "主管", reportsTo: "user", tool: "Claude Code", state: "ready" },
      { agentId: "r2", name: "研究员", role: "资料收集", reportsTo: "r1", tool: "Claude Code", state: "ready" },
    ],
  },
  {
    id: "tm3",
    name: "开发者团队",
    supervisorId: "r3",
    workspace: "core-dev",
    members: [
      { agentId: "r3", name: "分析师", role: "主管", reportsTo: "user", tool: "Codex CLI", state: "auth-required" },
    ],
  },
];

export const seedTasks = (): Task[] => [
  {
    id: "t1",
    name: "整理 Q3 竞品调研并生成对比报告",
    workspace: "research-2025",
    mode: "team",
    team: "调研团队 · 4 人",
    runId: "#1247",
    supervisor: "研究协调员",
    createdAt: taskCreatedAt(1),
    startedAt: "14:02",
    state: "running",
    stateNote: "已运行 12 分钟",
    subtasks: [
      { id: "s1", name: "拆解任务并分配角色", assignee: "研究协调员 · 主管", state: "success", result: "已生成 4 个子任务与依赖" },
      { id: "s2", name: "收集三家竞品的公开资料", assignee: "研究员 · Claude Code", state: "success", result: "12 个来源，含 3 个官方定价页" },
      { id: "s3", name: "生成对比表格草稿", assignee: "分析师 · Codex CLI", state: "running", result: "正在写入 docs/compare-draft.md" },
      { id: "s4", name: "撰写结论与风险提示", assignee: "写作者 · Claude Code", state: "waiting-dependency", result: "依赖 #3 完成后自动开始" },
      { id: "s5", name: "导出报告并写入工作空间", assignee: "写作者 · Claude Code", state: "waiting-approval", result: "需要一次性写入授权" },
    ],
    messages: [
      { id: uid("m"), author: "supervisor", authorName: "研究协调员 · 主管", time: "14:02", text: "已把任务拆成 4 个阶段：资料收集 → 对比表 → 结论 → 导出。资料收集已完成，对比表正在生成。第 4 步写入文件需要你的授权，我不会代替你同意。" },
      { id: uid("m"), author: "user", authorName: "你", time: "14:06", text: "优先保证来源可追溯。缺数据的地方留空并说明，不要推测。" },
    ],
    instruction: null,
  },
  {
    id: "t2",
    name: "批量规范化 320 条术语词表",
    workspace: "vocab-ops",
    mode: "team",
    team: "数据小组 · 2 人",
    runId: "#1244",
    supervisor: "数据协调员",
    createdAt: taskCreatedAt(4),
    startedAt: "12:55",
    state: "waiting-approval",
    stateNote: "1 项写入等待授权",
    subtasks: [
      { id: "s1", name: "读取 terms/ 全部词条", assignee: "数据 Agent", state: "success", result: "320 条 · 5 个文件" },
      { id: "s2", name: "规范化并覆盖写回", assignee: "数据 Agent", state: "waiting-approval", result: "需要写入授权" },
    ],
    messages: [
      { id: uid("m"), author: "supervisor", authorName: "数据协调员 · 主管", time: "13:10", text: "写入需要你确认。该目录同时被另一个任务使用，我已暂停派发新工作。" },
    ],
    instruction: null,
  },
  {
    id: "t3",
    name: "为更新日志生成中英文双版本文案",
    workspace: "release-2.4",
    mode: "team",
    team: "内容团队 · 3 人",
    runId: "#1241",
    supervisor: "研究协调员",
    createdAt: taskCreatedAt(9),
    startedAt: "13:48",
    state: "running",
    stateNote: "3/5 子任务完成",
    subtasks: [
      { id: "s1", name: "提取本次变更清单", assignee: "研究协调员", state: "success", result: "14 条变更" },
      { id: "s2", name: "生成中文初稿", assignee: "写作者", state: "success", result: "docs/changelog-zh.md" },
      { id: "s3", name: "生成英文初稿", assignee: "写作者", state: "success", result: "docs/changelog-en.md" },
      { id: "s4", name: "校对术语一致性", assignee: "研究员", state: "running", result: "正在比较 38 个术语" },
      { id: "s5", name: "合并与导出", assignee: "研究协调员", state: "waiting-dependency", result: "依赖 #4" },
    ],
    messages: [{ id: uid("m"), author: "supervisor", authorName: "研究协调员 · 主管", time: "13:50", text: "两个语言版本已生成，正在校对术语；校对完成后我会合并。" }],
    instruction: null,
  },
  {
    id: "t4",
    name: "重构 CLI 适配器的错误处理",
    workspace: "core-dev",
    mode: "team",
    team: "开发者团队 · 2 人",
    runId: "#1238",
    supervisor: "分析师",
    createdAt: taskCreatedAt(2),
    startedAt: "13:20",
    state: "queued",
    stateNote: "排队等待资源（并发 3/3）",
    subtasks: [
      { id: "s1", name: "阅读现有限流与重试逻辑", assignee: "分析师", state: "pending", result: "未开始" },
      { id: "s2", name: "重写错误分类", assignee: "分析师", state: "pending", result: "未开始" },
    ],
    messages: [{ id: uid("m"), author: "supervisor", authorName: "分析师 · 主管", time: "13:21", text: "当前并发已满，任务在队列中。不会丢弃，也不会假装已开始。" }],
    instruction: null,
  },
  {
    id: "t5",
    name: "整理供应商合同条款对比",
    workspace: "legal-docs",
    mode: "single-agent",
    team: "法务助手 · 单 Agent",
    runId: "#1245",
    supervisor: "无（单 Agent）",
    createdAt: taskCreatedAt(17),
    startedAt: "13:20",
    state: "failed",
    stateNote: "读取 PDF 时异常退出",
    subtasks: [
      { id: "s1", name: "读取 6 份合同 PDF", assignee: "法务助手", state: "failure", result: "PDF 编码无法解析，进程退出（exit 1）" },
      { id: "s2", name: "提取条款并对比", assignee: "法务助手", state: "blocked", result: "依赖 #1 失败，已阻塞" },
    ],
    messages: [{ id: uid("m"), author: "supervisor", authorName: "法务助手", time: "13:22", text: "读取第 3 份合同时进程退出。我停下等待你的选择：只重试失败部分，或跳过并生成标注完整性的部分成果。" }],
    instruction: null,
  },
  {
    id: "t6",
    name: "导出词汇覆盖率报告.csv",
    workspace: "vocab-ops",
    mode: "team",
    team: "数据小组 · 2 人",
    runId: "#1242",
    supervisor: "研究协调员",
    createdAt: taskCreatedAt(35),
    startedAt: "11:58",
    state: "done",
    stateNote: "产物已保存到 exports/",
    subtasks: [
      { id: "s1", name: "统计覆盖率", assignee: "研究协调员", state: "success", result: "覆盖率 87.4%" },
      { id: "s2", name: "导出 CSV", assignee: "研究协调员", state: "success", result: "exports/coverage-2025q3.csv" },
    ],
    messages: [{ id: uid("m"), author: "supervisor", authorName: "研究协调员 · 主管", time: "12:02", text: "报告已导出，覆盖率 87.4%。" }],
    instruction: null,
  },
];

export const seedApprovals = (): ApprovalRequest[] => [
  {
    id: "a1",
    taskId: "t1",
    state: "pending",
    age: "2 分钟前",
    title: "写入文件 /docs/report-q3.md",
    agent: "写作者 · Claude Code",
    target: "write  /docs/report-q3.md   (覆盖已存在文件 · 2.4 KB → 6.1 KB)",
    diff: "+ 新增 42 行   ~ 修改 6 行   - 删除 0 行",
    scope: "单个文件 · 单次 · 仅本 Run。不会扩展到其他文件或其他任务。",
    risk: ["目标文件已存在，写入会覆盖原有内容", "该文件不在本任务的默认写入白名单内", "若拒绝，Agent 会停在当前步骤，不会自动重试"],
    history: [
      { time: "14:04:02", title: "请求已发出", detail: "写作者 · Run #1247 · 已送达运行时" },
      { time: "14:04:02", title: "任务已暂停派发", detail: "等待用户决定；其他无依赖子任务不受影响" },
      { time: "现在", title: "等待你的决定", detail: "超时不会自动批准；2 小时后标记为已过期" },
    ],
  },
  {
    id: "a2",
    taskId: "t4",
    state: "pending",
    age: "18 分钟前",
    title: "执行命令 git push --force",
    agent: "分析师 · Codex CLI",
    target: "shell  git push --force origin main",
    diff: "重写远端 main 分支历史（3 个提交）",
    scope: "单条命令 · 单次 · 不可撤销。远端历史重写后无法自动回滚。",
    risk: ["会重写远端分支历史，其他人已拉取的提交会冲突", "该命令不在默认白名单内", "拒绝后任务停在当前步骤，不会重试"],
    history: [
      { time: "13:46:10", title: "请求已发出", detail: "分析师 · Run #1238" },
      { time: "13:46:10", title: "任务已暂停派发", detail: "等待用户决定" },
      { time: "现在", title: "等待你的决定", detail: "超过 2 小时将标记为已过期" },
    ],
  },
  {
    id: "a3",
    taskId: "t2",
    state: "expired",
    age: "2 小时前",
    title: "删除目录 terms/archive/",
    agent: "数据 Agent",
    target: "delete  terms/archive/   (38 个文件)",
    diff: "删除 38 个文件 · 约 1.2 MB",
    scope: "目录 · 单次 · 不可撤销",
    risk: ["删除操作不可撤销", "目录内包含历史版本"],
    history: [
      { time: "11:20:04", title: "请求已发出", detail: "数据 Agent" },
      { time: "13:20:04", title: "已过期", detail: "超时未处理；不会自动批准" },
    ],
  },
  {
    id: "a4",
    taskId: "t1",
    state: "denied",
    age: "1 小时前",
    title: "访问网络：竞品定价页",
    agent: "研究员 · Claude Code",
    target: "network  GET https://example-competitor.test/pricing",
    diff: "1 次请求 · 目标为公开页面",
    scope: "单次 · 仅研究 Agent",
    risk: ["会把内部术语表片段发送到外部页面参数"],
    history: [
      { time: "13:02:11", title: "请求已发出", detail: "研究员" },
      { time: "13:03:02", title: "已拒绝（你）", detail: "原因：不向外部发送内部术语" },
    ],
  },
];

export const seedRuns = (): RunRecord[] => [
  { id: "#1247", taskName: "整理 Q3 竞品调研并生成对比报告", executor: "调研团队", state: "running", startedAt: "14:02", duration: "12m" },
  { id: "#1246", taskName: "为更新日志生成中英文双版本文案", executor: "内容团队", state: "finished", startedAt: "13:48", duration: "6m" },
  { id: "#1245", taskName: "整理供应商合同条款对比", executor: "法务助手", state: "crashed", startedAt: "13:20", duration: "2m" },
  { id: "#1244", taskName: "批量规范化 320 条术语词表", executor: "数据小组", state: "finished", startedAt: "12:55", duration: "9m" },
  { id: "#1243", taskName: "统计术语覆盖率", executor: "数据小组", state: "stop-requested", startedAt: "12:30", duration: "4m" },
  { id: "#1242", taskName: "导出词汇覆盖率报告.csv", executor: "数据小组", state: "unknown", startedAt: "11:58", duration: "—" },
  { id: "#1241", taskName: "整理周会纪要与待办", executor: "内容团队", state: "finished", startedAt: "昨天", duration: "3m" },
];

export const seedWorkspaces = (): Workspace[] => [
  { id: "w1", name: "research-2025", path: "~/work/research-2025", taskCount: 3, state: "ready", read: ["terms/", "docs/", "exports/"], write: ["terms/", "exports/"], deny: ["~/.ssh", "~/.config", "其他工作空间"] },
  { id: "w2", name: "vocab-ops", path: "~/work/vocab-ops", taskCount: 5, state: "conflict", read: ["terms/", "docs/", "exports/"], write: ["terms/", "exports/"], deny: ["~/.ssh", "~/.config", "其他工作空间"] },
  { id: "w3", name: "release-2.4", path: "~/work/release-2.4", taskCount: 2, state: "ready", read: ["docs/", "changelog/"], write: ["docs/"], deny: ["src/", "~/.ssh"] },
  { id: "w4", name: "legal-docs", path: "~/work/legal-docs", taskCount: 1, state: "readonly", read: ["contracts/"], write: [], deny: ["全部写入操作"] },
];

export const seedNotifications = (): NotificationItem[] => [
  { id: "n1", kind: "blocked", title: "审批请求：写入 /docs/report-q3.md", body: "写作者 正在等待你的决定 · 任务 12 分钟后仍处于等待状态", time: "2 分钟前", unread: true, target: { screen: "approvals" } },
  { id: "n2", kind: "failure", title: "任务失败：整理供应商合同条款对比", body: "法务助手 在读取 PDF 时异常退出（Run #1245）", time: "25 分钟前", unread: true, target: { screen: "tasks", taskId: "t5" } },
  { id: "n3", kind: "neutral", title: "已暂停：统计术语覆盖率", body: "在安全检查点暂停，已有 2 项成果保留", time: "1 小时前", unread: true, target: { screen: "tasks", taskId: "t2" } },
  { id: "n4", kind: "success", title: "任务完成：导出词汇覆盖率报告.csv", body: "数据小组 · Run #1242 · 产物已保存到 exports/", time: "今天 15:41", unread: false, target: { screen: "history" } },
  { id: "n5", kind: "blocked", title: "审批已过期：删除目录 terms/archive/", body: "超过 2 小时未处理，已标记为过期；不会自动批准", time: "2 小时前", unread: false, target: { screen: "approvals" } },
];

export const seedLogs = (): LogLine[] => [
  { time: "14:02:07", level: "INFO", actor: "supervisor", message: "plan created · 4 subtasks · deps=[[1],[2],[3]]" },
  { time: "14:02:09", level: "INFO", actor: "researcher", message: "adapter=claude-code status=ready workdir=~/work/research-2025" },
  { time: "14:02:14", level: "INFO", actor: "researcher", message: "read 6 sources · 3 official pricing pages" },
  { time: "14:02:31", level: "WARN", actor: "researcher", message: "source#3 http 403 · retry 1/1 after 20s" },
  { time: "14:03:02", level: "INFO", actor: "analyst", message: "adapter=codex-cli status=running model=default" },
  { time: "14:03:04", level: "INFO", actor: "analyst", message: "write docs/compare-draft.md · 2.4 KB" },
  { time: "14:03:18", level: "WARN", actor: "analyst", message: "table row 7 missing price_cny · left blank per user rule" },
  { time: "14:03:40", level: "INFO", actor: "analyst", message: "diff +38 / -0 lines" },
  { time: "14:03:55", level: "INFO", actor: "writer", message: "waiting dependency: node#3" },
  { time: "14:04:02", level: "INFO", actor: "writer", message: "approval requested · write /docs/report-q3.md" },
  { time: "14:04:02", level: "INFO", actor: "system", message: "task state → waiting_approval · dispatch paused" },
  { time: "14:04:15", level: "INFO", actor: "supervisor", message: "notified user · 1 decision pending" },
  { time: "14:04:22", level: "DEBUG", actor: "adapter", message: "heartbeat ok · queue=1 · concurrency 2/3" },
  { time: "14:04:40", level: "INFO", actor: "analyst", message: "stream idle 38s · no output" },
];

export const seedPartialGroups = () => [
  { kind: "success" as const, label: "已完成", items: [
    { path: "terms/a-in.md", note: "规范化完成" },
    { path: "terms/j-r.md", note: "规范化完成" },
    { path: "terms/s-z.md", note: "规范化完成" },
  ]},
  { kind: "neutral" as const, label: "跳过", items: [{ path: "terms/archive/", note: "等待授权超时，未处理" }] },
  { kind: "failure" as const, label: "失败", items: [{ path: "terms/import-raw.csv", note: "编码无法识别，已保留原文件" }] },
];

/** 内存数据库：各 Service 共享，保证跨页面一致。 */
export class MockDb {
  agents = seedAgents();
  tools = seedTools();
  teams = seedTeams();
  tasks = seedTasks();
  approvals = seedApprovals();
  runs = seedRuns();
  workspaces = seedWorkspaces();
  notifications = seedNotifications();
  logs = seedLogs();
  partialGroups = seedPartialGroups();
  settings: AppSettings = {
    theme: "system",
    density: "comfortable",
    grayscalePreview: false,
    reducedMotion: false,
    platform: "mac",
    closeBehavior: "window",
    concurrency: 3,
    autoRetry: false,
    logRetention: "30 天",
    notificationPrefs: { approval: true, failure: true, done: false, systemOnly: false },
  };
  defaultToolId = "claude";
  extraRuns = 0;

  /** 每创建一个任务/Run 都递增，避免出现重复编号。 */
  nextRunId(): string {
    return `#${1248 + Math.floor(Math.random() * 40)}`;
  }
}

export const newId = uid;
