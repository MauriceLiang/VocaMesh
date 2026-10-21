import type { ApprovalState, AgentState, RunState, SubtaskState, TaskState, ToolState } from "@/types";
import type { StatusTone } from "@/types";

/**
 * 状态 → 视觉语义的唯一映射。
 *
 * 规则（来自 design-system/components.md §4）：
 * - 成功=绿、失败=红、阻塞=琥珀；运行中/排队/暂停/未知=中性灰。
 * - 页面不得自行决定颜色，只能使用这里的 tone。
 */
export interface StatusMeta {
  label: string;
  tone: StatusTone;
  /** 图标键，由 VocaStatus 映射到具体图标组件。 */
  icon: "check" | "x" | "alert" | "dot" | "loader" | "help" | "pause" | "square" | "hourglass" | "shield";
}

export const taskStateMeta = (state: TaskState): StatusMeta => {
  switch (state) {
    case "running": return { label: "执行中", tone: "neutral", icon: "loader" };
    case "queued": return { label: "排队中", tone: "neutral", icon: "hourglass" };
    case "waiting-approval": return { label: "等待审批", tone: "blocked", icon: "alert" };
    case "pause-pending": return { label: "等待暂停", tone: "neutral", icon: "pause" };
    case "paused": return { label: "已暂停", tone: "neutral", icon: "pause" };
    case "stop-pending": return { label: "正在请求停止", tone: "unknown", icon: "loader" };
    case "stopped": return { label: "已停止", tone: "neutral", icon: "square" };
    case "needs-attention": return { label: "需要处理", tone: "blocked", icon: "alert" };
    case "partial": return { label: "部分完成", tone: "blocked", icon: "alert" };
    case "done": return { label: "已完成", tone: "success", icon: "check" };
    case "failed": return { label: "执行失败", tone: "failure", icon: "x" };
    case "draft": return { label: "草稿", tone: "neutral", icon: "dot" };
    default: return { label: "状态待确认", tone: "unknown", icon: "help" };
  }
};

export const subtaskStateMeta = (state: SubtaskState): StatusMeta => {
  switch (state) {
    case "success": return { label: "成功", tone: "success", icon: "check" };
    case "failure": return { label: "失败", tone: "failure", icon: "x" };
    case "running": return { label: "执行中", tone: "neutral", icon: "loader" };
    case "waiting-approval": return { label: "等待审批", tone: "blocked", icon: "shield" };
    case "blocked": return { label: "已阻塞", tone: "blocked", icon: "alert" };
    case "waiting-dependency": return { label: "等待依赖", tone: "neutral", icon: "hourglass" };
    case "skipped": return { label: "已跳过", tone: "neutral", icon: "dot" };
    case "cancelled": return { label: "已取消", tone: "neutral", icon: "x" };
    case "ready": return { label: "准备执行", tone: "neutral", icon: "dot" };
    case "pending": return { label: "未开始", tone: "neutral", icon: "dot" };
    default: return { label: "状态未知", tone: "unknown", icon: "help" };
  }
};

export const toolStateMeta = (state: ToolState): StatusMeta => {
  switch (state) {
    case "ready": return { label: "已安装且可用", tone: "success", icon: "check" };
    case "pending": return { label: "待验证", tone: "neutral", icon: "dot" };
    case "auth-required": return { label: "认证待验证", tone: "blocked", icon: "alert" };
    case "missing": return { label: "未安装", tone: "neutral", icon: "dot" };
    case "incompatible": return { label: "不兼容", tone: "failure", icon: "x" };
    default: return { label: "无法检测", tone: "unknown", icon: "help" };
  }
};

export const agentStateMeta = (state: AgentState): StatusMeta => {
  switch (state) {
    case "ready": return { label: "可用", tone: "success", icon: "check" };
    case "auth-required": return { label: "需登录", tone: "blocked", icon: "alert" };
    case "not-ready": return { label: "未就绪", tone: "unknown", icon: "help" };
    default: return { label: "待验证", tone: "neutral", icon: "dot" };
  }
};

export const runStateMeta = (state: RunState): StatusMeta => {
  switch (state) {
    case "running": return { label: "执行中", tone: "neutral", icon: "loader" };
    case "finished": return { label: "已完成", tone: "success", icon: "check" };
    case "crashed": return { label: "异常退出", tone: "failure", icon: "x" };
    case "stop-requested": return { label: "已停止", tone: "neutral", icon: "square" };
    case "starting": return { label: "启动中", tone: "neutral", icon: "loader" };
    case "waiting-input": return { label: "等待输入", tone: "blocked", icon: "alert" };
    case "pending": return { label: "待启动", tone: "neutral", icon: "dot" };
    default: return { label: "状态未知", tone: "unknown", icon: "help" };
  }
};

export const approvalStateMeta = (state: ApprovalState): StatusMeta => {
  switch (state) {
    case "pending": return { label: "等待审批", tone: "blocked", icon: "alert" };
    case "allowed-once": return { label: "已允许一次", tone: "success", icon: "check" };
    case "allowed-task": return { label: "任务内授权", tone: "success", icon: "check" };
    case "denied": return { label: "已拒绝", tone: "failure", icon: "x" };
    case "expired": return { label: "已过期", tone: "neutral", icon: "dot" };
    case "revoked": return { label: "已撤销", tone: "neutral", icon: "dot" };
    default: return { label: "无法确定", tone: "unknown", icon: "help" };
  }
};
