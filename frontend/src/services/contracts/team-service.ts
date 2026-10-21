import type { Team, TeamMember } from "@/types";

/**
 * Agent 团队与组织关系服务。
 *
 * 用途：维护“谁向谁汇报”的组织关系（与任务依赖无关）。
 * 当前实现：MockTeamService。
 * 未来实现：Rust Core 的团队配置持久化。
 */
export interface TeamService {
  /** 列出全部团队。调用时机：S09 左树、创建任务选择团队。 */
  listTeams(): Promise<Team[]>;

  /** 读取单个团队。错误：`team.not-found`。 */
  getTeam(id: string): Promise<Team>;

  /**
   * 创建团队。
   *
   * 调用时机：S09「新建团队」保存。
   * 说明：必须指定主管；主管默认向用户汇报。
   * 错误：重名 `team.duplicate`。
   */
  createTeam(input: { name: string; supervisorId: string; workspace: string }): Promise<Team>;

  /** 更新团队基础信息（名称、主管、默认工作空间）。错误：`team.not-found`、`team.cycle`。 */
  updateTeam(id: string, input: Partial<Pick<Team, "name" | "supervisorId" | "workspace">>): Promise<Team>;

  /**
   * 添加成员。
   *
   * 调用时机：S09「添加成员」、关系图节点 `+`、`+ 添加 Agent`。
   * 限制：不能把主管加为自己的下属（自我管理）；不能重复添加。
   * 错误：`team.self-management`、`team.member-exists`。
   */
  addMember(teamId: string, input: { agentId: string; reportsTo: string }): Promise<Team>;

  /** 移除成员。限制：主管不可直接移除，需先指定新主管。错误：`team.supervisor-required`。 */
  removeMember(teamId: string, agentId: string): Promise<Team>;

  /**
   * 设置成员的直属上级。
   *
   * 调用时机：关系图「添加直属上级…」、树形编辑。
   * 限制：禁止自我管理；禁止间接环（A→B→A）。`reportsTo: "user"` 表示成为团队主管。
   * 错误：`team.self-management`、`team.cycle`。
   * Mock：返回 `ServiceError("team.cycle", path)`，`detail` 携带环路径用于提示。
   * Rust：校验并写入，需在事务内完成以免并发写出环。
   */
  setReportsTo(teamId: string, agentId: string, reportsTo: string): Promise<Team>;

  /** 读取团队成员（含展示所需的名字/工具/状态）。 */
  listMembers(teamId: string): Promise<TeamMember[]>;
}
