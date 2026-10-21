import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { Workspace } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

/** 工作空间 Store：目录与读写边界。 */
export const useWorkspacesStore = defineStore("workspaces", () => {
  const services = useServices();
  const app = useAppStore();

  const workspaces = ref<Workspace[]>([]);
  const selectedId = ref<string | null>(null);
  const loading = ref(false);

  const selected = computed<Workspace | null>(() => workspaces.value.find((item) => item.id === selectedId.value) ?? null);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      workspaces.value = await services.workspaces.listWorkspaces();
      if (!selectedId.value && workspaces.value.length) selectedId.value = workspaces.value[0].id;
    } finally {
      loading.value = false;
    }
  }

  function select(id: string): void {
    selectedId.value = id;
  }

  async function create(name: string, path: string): Promise<boolean> {
    try {
      const workspace = await services.workspaces.createWorkspace({ name, path });
      workspaces.value = [...workspaces.value, workspace];
      selectedId.value = workspace.id;
      app.toast(`已添加工作空间 ${workspace.name}（写入范围默认关闭）`, "success");
      return true;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "添加失败", "failure");
      return false;
    }
  }

  async function updateBoundaries(read: string[], write: string[]): Promise<boolean> {
    if (!selected.value) return false;
    try {
      const workspace = await services.workspaces.updateBoundaries(selected.value.id, { read, write });
      workspaces.value = workspaces.value.map((item) => (item.id === workspace.id ? workspace : item));
      app.toast("访问边界已保存（从下一个任务生效）", "success");
      return true;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "保存失败", "failure");
      return false;
    }
  }

  async function resolveConflict(): Promise<void> {
    if (!selected.value) return;
    try {
      const workspace = await services.workspaces.resolveConflict(selected.value.id);
      workspaces.value = workspaces.value.map((item) => (item.id === workspace.id ? workspace : item));
      app.toast("已将其中一个 Run 排队，等待当前写入完成", "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "处理失败", "failure");
    }
  }

  async function resolveOpenPath(): Promise<string> {
    if (!selected.value) return "";
    return services.workspaces.resolveOpenPath(selected.value.id);
  }

  return { workspaces, selectedId, selected, loading, load, select, create, updateBoundaries, resolveConflict, resolveOpenPath };
});
