import type { TaskFilterState, TaskTimeFilter } from "@/types";

/** Dropdown labels stay in Chinese; Store values use domain state codes. */
export const taskStateFilterOptions = [
  { label: "全部", value: "all" },
  { label: "执行中", value: "running" },
  { label: "等待审批", value: "waiting-approval" },
  { label: "待配置", value: "draft" },
  { label: "排队中", value: "queued" },
  { label: "已暂停", value: "paused" },
  { label: "需要处理", value: "needs-attention" },
  { label: "已完成", value: "done" },
  { label: "失败", value: "failed" },
] satisfies ReadonlyArray<{ label: string; value: TaskFilterState }>;

export const taskTimeFilterOptions = [
  { label: "最近 7 天", value: "7d" },
  { label: "最近 30 天", value: "30d" },
  { label: "全部时间", value: "all" },
] satisfies ReadonlyArray<{ label: string; value: TaskTimeFilter }>;
