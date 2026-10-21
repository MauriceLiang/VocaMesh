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
  const resultFilter = ref("全部");
  const loading = ref(false);

  const selected = computed<RunRecord | null>(() => runs.value.find((run) => run.id === selectedId.value) ?? runs.value[0] ?? null);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      const { items, hasMore: more } = await services.runs.listRuns({ result: resultFilter.value });
      runs.value = items;
      hasMore.value = more;
      if (!selectedId.value && items.length) selectedId.value = items[0].id;
      partialGroups.value = await services.runs.getPartialArtifacts(items[0]?.id ?? "#1244");
    } finally {
      loading.value = false;
    }
  }

  async function loadMore(): Promise<void> {
    const { items } = await services.runs.listRuns({ result: resultFilter.value });
    runs.value = items;
    app.toast("已加载更早的 Run（演示）", "neutral");
  }

  function select(id: string): void {
    selectedId.value = id;
  }

  async function exportCsv(): Promise<void> {
    try {
      const csv = await services.runs.exportRunsCsv({ result: resultFilter.value });
      downloadText("vocamesh-runs.csv", csv, "text/csv;charset=utf-8");
      app.toast("已生成 vocamesh-runs.csv（演示内容）", "success");
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

  return { runs, selectedId, selected, partialGroups, hasMore, resultFilter, loading, load, loadMore, select, exportCsv, exportPartial };
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
