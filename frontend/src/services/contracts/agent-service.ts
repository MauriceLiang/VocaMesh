import type { Agent, AgentPermissions, CreateAgentInput, UpdateAgentInput } from "@/types";

/**
 * Agent 角色服务。
 *
 * 用途：
 * 统一管理本地 Agent 角色（名称、职责、执行工具、权限、指令）。
 *
 * Browser Mock 使用内存数据；Tauri 通过 Rust Catalog 持久化角色配置。保存配置不会授予 Runtime 权限或启动进程。
 *
 * 注意：
 * Agent 角色与本地执行工具（CLI）是不同概念：一个工具可支撑多个角色。
 */
export interface AgentService {
  /**
   * 获取全部 Agent 角色。
   *
   * 调用时机：进入“Agent 角色”页、创建任务时选择执行者、团队关系图加载时。
   * 返回：角色集合（按创建顺序）。
   * 错误：`ServiceError("agent.list-failed")`。
   * Mock：返回固定种子数据 + 本会话内新建的角色。
   * Tauri：读取持久化角色；执行状态独立于工具探测和 managed policy。
   */
  listAgents(): Promise<Agent[]>;

  /**
   * 创建 Agent 角色。
   *
   * 调用时机：S08 新建角色、关系图“+ 添加 Agent”。
   * 参数：名称、职责、执行工具、可选描述/指令/权限。
   * 返回：已创建角色（状态由所选工具推导：可用 / 需登录 / 未就绪）。
   * 错误：重名 `ServiceError("agent.duplicate")`、参数非法 `agent.invalid-input`。
   * Mock：写入内存并同步到团队（若从关系图创建）。
   * Tauri：持久化配置，不启动任何进程；保存成功不代表角色可执行。
   */
  createAgent(input: CreateAgentInput): Promise<Agent>;

  /**
   * 更新角色配置。
   *
   * 调用时机：S08 保存修改、关系图编辑节点。
   * 限制：只影响后续新任务/新 Run，不得静默改变正在运行的 Run。
   * 错误：不存在 `agent.not-found`、重名 `agent.duplicate`。
   * Mock：就地更新并广播变更事件。
   * Tauri：持久化 Catalog 配置版本；当前 Run 执行尚未消费 Agent 配置。
   */
  updateAgent(id: string, input: UpdateAgentInput): Promise<Agent>;

  /**
   * 删除角色。
   *
   * 调用时机：S08 删除（二次确认后）。
   * 限制：历史 Run / 审批记录/日志保留；若角色是团队主管，需先转移主管。
   * 错误：`agent.in-use`（主管或运行中）、`agent.not-found`。
   * Mock：从角色与团队成员中移除。
   * Tauri：持久化删除并保留审计引用；被团队引用的角色不能删除。
   */
  deleteAgent(id: string): Promise<void>;

  /**
   * 更新单个权限开关。
   *
   * 调用时机：S08 权限行开关。
   * 说明：拆分出来的原因是权限变更需要独立审计。
   * Tauri：持久化角色权限草稿并记录审计；保存不会向运行时授予权限。
   */
  setPermissions(id: string, permissions: Partial<AgentPermissions>): Promise<Agent>;
}
