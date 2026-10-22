import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { RunArtifactGroup, RunRecord } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

/** 历史记录（Run）Store：只读 + 导出；重试通过任务 Store 触发。 */
export const useRunsStore = defineStore("runs", () => {
  const services = useServices();
  const app = useAppStore();

  const runs = ref<RunRecord[]>([]);
  const selectedId = ref<string | null>(null);
  const partialGroups = ref<RunArtifactGroup[]>([]);
  const hasMore = ref(false);
  const nextCursor = ref<string | null>(null);
  const resultFilter = ref("全部");
  const loading = ref(false);

  const selected = computed<RunRecord | null>(() => runs.value.find((run) => run.id === selectedId.value) ?? runs.value[0] ?? null);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      const { items, hasMore: more, nextCursor: cursor } = await services.runs.listRuns({ result: resultFilter.value });
      runs.value = items;
      hasMore.value = more;
      nextCursor.value = cursor ?? null;
      if (!selectedId.value || !items.some((item) => item.id === selectedId.value)) selectedId.value = items[0]?.id ?? null;
      partialGroups.value = items.length ? await services.runs.getPartialArtifacts(selectedId.value ?? items[0].id) : [];
    } finally {
      loading.value = false;
    }
  }

  async function loadMore(): Promise<void> {
    if (!nextCursor.value) return;
    const { items, hasMore: more, nextCursor: cursor } = await services.runs.listRuns({ result: resultFilter.value, cursor: nextCursor.value });
    const known = new Set(runs.value.map((run) => run.id));
    runs.value = [...runs.value, ...items.filter((run) => !known.has(run.id))];
    hasMore.value = more;
    nextCursor.value = cursor ?? null;
    if (services.mode === "mock") app.toast("已加载更早的 Run（演示）", "neutral");
  }

  function select(id: string): void {
    selectedId.value = id;
  }

  async function exportCsv(): Promise<void> {
    try {
      const csv = await services.runs.exportRunsCsv({ result: resultFilter.value });
      downloadText("vocamesh-runs.csv", csv, "text/csv;charset=utf-8");
      app.toast(services.mode === "mock" ? "已生成 vocamesh-runs.csv（演示内容）" : "已导出本机 Run 记录", "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "导出失败", "failure");
    }
  }

  async function exportPartial(): Promise<void> {
    const rows = ["path,status,note"];
    partialGroups.value.forEach((group) => group.items.forEach((item) => rows.push(`${item.path},${group.label},"${item.note}"`)));
    downloadText("run-1244-partial.csv", rows.join("\n"), "text/csv;charset=utf-8");
    app.toast("已导出部分成果清单", "success");
  }

  return { runs, selectedId, selected, partialGroups, hasMore, nextCursor, resultFilter, loading, load, loadMore, select, exportCsv, exportPartial };
});

/** 触发浏览器下载（Mock 阶段唯一的“真实”副作用）。 */
export function downloadText(filename: string, content: string, mime: string): void {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  window.setTimeout(() => URL.revokeObjectURL(url), 2000);
}
