import type { ResultArtifact, RunArtifactGroup, RunRecord } from "@/types";

/**
 * 历史记录 / Run 服务。
 *
 * 用途：追溯执行尝试、导出记录、读取部分成果。
 * 当前实现：MockRunService。
 * 未来实现：Rust Core 的 Run 存储（不可覆盖，重试产生新 Run）。
 */
export interface RunService {
  /**
   * 列出 Run。
   *
   * 调用时机：S12。
   * 参数：可选过滤（类型/结果/时间）与分页游标。
   * 返回：Run 列表 + 是否还有更多。
   * Mock：返回固定 7 条，`loadMore` 追加演示数据。
   */
  listRuns(filter?: { type?: string; result?: string; time?: string; cursor?: string }): Promise<{ items: RunRecord[]; hasMore: boolean }>;

  /** 读取 Run 的部分成果分组（完成/跳过/失败）。错误：`run.not-found`。 */
  getPartialArtifacts(runId: string): Promise<RunArtifactGroup[]>;

  /** 读取 Run 的产物清单。 */
  listArtifacts(runId: string): Promise<ResultArtifact[]>;

  /**
   * 导出记录（CSV）。
   *
   * 调用时机：S12「导出记录」。
   * 返回：CSV 文本；由 UI 触发下载。
   * 错误：`run.export-failed`。
   * Mock：由内存数据拼接。
   */
  exportRunsCsv(filter?: { type?: string; result?: string }): Promise<string>;
}
