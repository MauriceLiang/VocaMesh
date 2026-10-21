import type { ApprovalRequest, LogLine, Task } from "@/types";

/**
 * 事件订阅服务。
 *
 * 用途：日志流、任务状态、审批变化、工具扫描进度等实时数据。
 * 当前实现：MockEventService（内存 EventTarget + 定时器）。
 * 未来实现：Tauri 事件（`listen("task://state")` 等）。
 *
 * 重要：实时数据**不是** HTTP 轮询模型；界面只订阅一次并按事件增量更新。
 * 未知命令名称一律标注“待实现职责”，不要虚构具体命令。
 */
export interface EventService {
  /**
   * 订阅任务执行日志。
   *
   * 调用时机：打开检查器「日志」Tab。
   * 返回：取消订阅函数（离开 Tab/页面必须调用，避免泄漏）。
   * Mock：首次回放 14 行历史，然后每 3s 追加一条心跳（明确标注为演示）。
   * Rust：由适配器把 CLI stdout/stderr 归一化为 LogLine；需处理背压与截断。
   */
  subscribeTaskLogs(taskId: string, handler: (line: LogLine) => void): () => void;

  /** 订阅任务状态变化（queued/running/paused/...）。 */
  subscribeTaskState(handler: (task: Task) => void): () => void;

  /** 订阅审批变化（新增、已决定、已过期）。 */
  subscribeApprovals(handler: (approval: ApprovalRequest) => void): () => void;

  /**
   * 订阅工具扫描进度。
   *
   * 调用时机：S01/S07 扫描期间。
   * Rust：可上报“正在探测第 N 个适配器”，避免界面用假进度。
   */
  subscribeToolScan(handler: (event: { phase: "scanning" | "done"; message: string }) => void): () => void;
}
