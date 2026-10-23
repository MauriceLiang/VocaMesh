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
  const lastError = ref<string | null>(null);

  const selected = computed<Workspace | null>(() => workspaces.value.find((item) => item.id === selectedId.value) ?? null);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      workspaces.value = await services.workspaces.listWorkspaces();
      lastError.value = null;
      if (!selectedId.value && workspaces.value.length) selectedId.value = workspaces.value[0].id;
      if (selectedId.value && !workspaces.value.some((item) => item.id === selectedId.value)) {
        selectedId.value = workspaces.value[0]?.id ?? null;
      }
    } catch (err) {
      lastError.value = err instanceof ServiceError ? err.message : "无法读取本机工作空间配置，请重试。";
      app.toast(lastError.value, "failure");
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
      app.toast(services.mode === "tauri" ? `已登记工作空间 ${workspace.name}；未授予执行权限` : `已添加工作空间 ${workspace.name}（写入范围默认关闭）`, "success");
      return true;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "添加失败", "failure");
      return false;
    }
  }

  async function updateBoundaries(read: string[], write: string[]): Promise<boolean> {
    if (!selected.value) return false;
    try {
      const workspace = await services.workspaces.updateBoundaries(selected.value.id, {
        read,
        write,
        expectedRevision: selected.value.revision,
      });
      workspaces.value = workspaces.value.map((item) => (item.id === workspace.id ? workspace : item));
      app.toast(services.mode === "tauri" ? "范围配置草稿已保存；当前 Runtime 权限未改变" : "访问边界已保存（从下一个任务生效）", "success");
      return true;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "保存失败", "failure");
      return false;
    }
  }

  async function rename(name: string): Promise<boolean> {
    if (!selected.value) return false;
    try {
      const workspace = await services.workspaces.renameWorkspace(selected.value.id, name, selected.value.revision);
      workspaces.value = workspaces.value.map((item) => (item.id === workspace.id ? workspace : item));
      lastError.value = null;
      app.toast("工作空间名称已保存；团队关系保持不变", "success");
      return true;
    } catch (err) {
      const message = err instanceof ServiceError ? err.message : "重命名工作空间失败";
      lastError.value = message;
      app.toast(message, "failure");
      return false;
    }
  }

  async function archive(id: string): Promise<boolean> {
    try {
      await services.workspaces.archiveWorkspace(id);
      workspaces.value = workspaces.value.filter((item) => item.id !== id);
      if (selectedId.value === id) selectedId.value = workspaces.value[0]?.id ?? null;
      lastError.value = null;
      app.toast("工作空间已归档；历史任务记录保留", "success");
      return true;
    } catch (err) {
      const message = err instanceof ServiceError ? err.message : "归档工作空间失败";
      lastError.value = message;
      app.toast(message, "failure");
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

  return { workspaces, selectedId, selected, loading, lastError, load, select, create, rename, updateBoundaries, archive, resolveConflict, resolveOpenPath };
});
