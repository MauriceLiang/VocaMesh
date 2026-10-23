import type { Workspace } from "@/types";

/**
 * 工作空间服务。
 *
 * 用途：定义本地目录登记与范围配置。
 * Browser Mock 使用内存数据；Tauri 通过 Rust Catalog 持久化真实目录。目录登记和范围草稿都不授予 Runtime 文件权限。
 */
export interface WorkspaceService {
  /** 列出工作空间。调用时机：S10、S03 工作空间选择。 */
  listWorkspaces(): Promise<Workspace[]>;

  /** 创建工作空间。错误：`workspace.duplicate`、`workspace.path-missing`。 */
  createWorkspace(input: { name: string; path: string }): Promise<Workspace>;

  /** Rename the local registration without changing its stable ID or directory. */
  renameWorkspace(id: string, name: string, expectedRevision?: number): Promise<Workspace>;

  /** 更新读写范围草稿。Tauri 通过 expectedRevision 避免覆盖较新的配置；草稿不授予 Runtime 权限。 */
  updateBoundaries(id: string, input: { read: string[]; write: string[]; expectedRevision?: number }): Promise<Workspace>;

  /** Archive a local registration. The native backend blocks this while a Team references it. */
  archiveWorkspace(id: string): Promise<void>;

  /** 解决演示目录冲突。Tauri 当前返回 feature.not-implemented。 */
  resolveConflict(id: string): Promise<Workspace>;

  /** 打开目录（真实实现由宿主执行）。Mock：仅返回路径字符串。 */
  resolveOpenPath(id: string): Promise<string>;
}
