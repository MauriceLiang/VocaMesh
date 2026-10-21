import type { ApprovalDecision, ApprovalRequest, ApprovalState } from "@/types";

/**
 * 审批服务。
 *
 * 用途：高风险操作的授权（覆盖写入、删除、命令执行、访问网络）。
 * 当前实现：MockApprovalService。
 * 未来实现：Rust Core 的授权引擎；必须支持作用域与撤销能力判定。
 *
 * 约束：
 * - 默认不允许；超时不会变成批准。
 * - 拒绝后不得被主管绕过或自动重试。
 * - 任务内审批卡与“待审批”页共享同一条记录（单份数据）。
 */
export interface ApprovalService {
  /** 列出审批记录。调用时机：S11、S02 待审批栏、S15 通知跳转。 */
  listApprovals(): Promise<ApprovalRequest[]>;

  /** 读取单条审批。错误：`approval.not-found`。 */
  getApproval(id: string): Promise<ApprovalRequest>;

  /**
   * 提交决定。
   *
   * 调用时机：S11 / S04 审批卡。
   * 参数：allow-once（仅此一次）或 deny。
   * 返回：更新后的审批记录（含 history 追加条目）。
   * 错误：`approval.already-decided`、`approval.expired`、`approval.scope-unavailable`。
   * Mock：允许 → 关联任务恢复 running；拒绝 → 任务 needs-attention。
   * Rust：写入审计记录；验证作用域仍然有效后才放行执行。
   */
  decide(id: string, decision: ApprovalDecision): Promise<ApprovalRequest>;

  /**
   * 查询是否具备“任务内授权 + 撤销”能力。
   *
   * 调用时机：渲染更宽授权按钮前。
   * 返回：false 时界面必须渲染禁用的“规划中”占位，而不是可选按钮。
   */
  canGrantTaskScope(): Promise<boolean>;

  /** 订阅审批变化（新增/决定/过期）。返回取消订阅函数。 */
  subscribe(handler: (approval: ApprovalRequest, state: ApprovalState) => void): () => void;
}
