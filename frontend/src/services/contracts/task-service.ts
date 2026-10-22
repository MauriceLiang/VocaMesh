import type { CreateTaskInput, Subtask, Task, TaskState } from "@/types";

/**
 * 任务服务。
 *
 * 用途：任务创建、状态迁移、子任务、暂停/停止/继续/重试与部分成果。
 * 当前实现：MockTaskService（定时器驱动状态迁移，不代表真实执行）。
 * 未来实现：Rust Core 的任务编排；所有状态必须由运行时确认，不得前端推测。
 */
export interface TaskService {
  /** 列出任务。调用时机：S02/S06。 */
  listTasks(): Promise<Task[]>;

  /** 读取任务。错误：`task.not-found`。 */
  getTask(id: string): Promise<Task>;

  /**
   * 创建任务。
   *
   * 调用时机：S02 输入区提交、S03 弹窗提交。
   * 返回：状态为 queued 的任务（创建 ≠ 开始执行）。
   * 错误：无可用执行者 `task.no-executor`、工作空间不存在 `task.workspace-missing`。
   * Mock：团队/单 Agent 模式在 1.4s 后迁移到 running；自定义占位保持 draft，直到计划编辑能力实现。
   * Rust：入队 + 运行环境检查后再派发；失败要给出可解释原因。
   */
  createTask(input: CreateTaskInput): Promise<Task>;

  /**
   * 暂停任务（安全边界暂停）。
   *
   * 调用时机：S04「暂停」。
   * 语义：先停止派发新工作 → 当前执行到达安全边界 → 才可进入 paused。
   * 错误：`task.illegal-transition`。
   * Mock：先 pause-pending，1.7s 后 paused。
   * Rust：向运行时发送暂停信号并等待确认；未确认不得显示“已暂停”。
   */
  pauseTask(id: string): Promise<Task>;

  /** 继续任务。调用时机：S04「继续」。恢复前需检查权限/依赖/输入版本。错误：`task.resume-blocked`。 */
  resumeTask(id: string): Promise<Task>;

  /**
   * 停止任务。
   *
   * 调用时机：S04「停止」。
   * 语义：请求 ≠ 结果；需运行时确认，超时进入 unknown。
   * Mock：stop-pending → 1.8s → stopped。
   */
  stopTask(id: string): Promise<Task>;

  /**
   * 仅重试失败部分。
   *
   * 调用时机：S04「仅重试失败部分」、S12「在新 Run 中重试」。
   * 语义：创建**新 Run**，旧 Run 的日志与产物保持不变。
   * 返回：携带新 runId 的任务。
   * 错误：`task.retry-unavailable`。
   */
  retryFailed(id: string): Promise<Task>;

  /**
   * 跳过失败/阻塞步骤并生成部分成果。
   *
   * 调用时机：S04「跳过并生成部分成果」。
   * 说明：任务变为 partial，必须在成果中标注缺失范围。
   */
  skipAndFinalizePartial(id: string): Promise<Task>;

  /** 读取子任务。调用时机：检查器定位。 */
  listSubtasks(taskId: string): Promise<Subtask[]>;

  /** 订阅任务状态变化（用于页面即时刷新）。见 EventService。 */
  subscribeTaskState(taskId: string, handler: (state: TaskState, task: Task) => void): () => void;
}
