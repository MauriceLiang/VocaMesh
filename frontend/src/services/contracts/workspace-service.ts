import type { Workspace } from "@/types";

/**
 * 工作空间服务。
 *
 * 用途：定义任务的本地目录与读写边界。
 * 当前实现：MockWorkspaceService（不访问真实文件系统）。
 * 未来实现：Rust Core 的目录授权；必须校验路径真实存在且在白名单内。
 */
export interface WorkspaceService {
  /** 列出工作空间。调用时机：S10、S03 工作空间选择。 */
  listWorkspaces(): Promise<Workspace[]>;

  /** 创建工作空间。错误：`workspace.duplicate`、`workspace.path-missing`。 */
  createWorkspace(input: { name: string; path: string }): Promise<Workspace>;

  /** 更新读写边界。说明：只影响后续新任务；受禁止清单约束。错误：`workspace.invalid-scope`。 */
  updateBoundaries(id: string, input: { read: string[]; write: string[] }): Promise<Workspace>;

  /** 解决目录冲突（排队其一）。错误：`workspace.no-conflict`。 */
  resolveConflict(id: string): Promise<Workspace>;

  /** 打开目录（真实实现由宿主执行）。Mock：仅返回路径字符串。 */
  resolveOpenPath(id: string): Promise<string>;
}
