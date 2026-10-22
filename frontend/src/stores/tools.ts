import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { LocalTool } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

/** Local runtime availability, kept separate from Agent role configuration. */
export const useToolsStore = defineStore("tools", () => {
  const services = useServices();
  const app = useAppStore();

  const tools = ref<LocalTool[]>([]);
  const loading = ref(false);
  const scanning = ref(false);
  const scanMessage = ref("");
  const defaultToolId = ref<string | null>(null);
  const error = ref<string | null>(null);
  let scanRevision = 0;

  const counts = computed(() => ({
    ready: tools.value.filter((tool) => tool.state === "ready").length,
    authRequired: tools.value.filter((tool) => tool.state === "auth-required").length,
    missing: tools.value.filter((tool) => tool.state === "missing").length,
    undetectable: tools.value.filter((tool) => tool.state === "undetectable").length,
  }));

  async function load(): Promise<void> {
    const revision = ++scanRevision;
    loading.value = true;
    scanning.value = true;
    scanMessage.value = "正在扫描本机工具";
    error.value = null;
    try {
      const snapshot = await services.tools.scanTools();
      const defaultId = await services.tools.getDefaultToolId();
      if (revision !== scanRevision) return;
      tools.value = snapshot;
      defaultToolId.value = defaultId;
    } catch (err) {
      if (revision !== scanRevision) return;
      error.value = err instanceof ServiceError ? err.message : "工具检测失败";
      app.toast(error.value, "failure");
    } finally {
      if (revision === scanRevision) {
        loading.value = false;
        scanning.value = false;
      }
    }
  }

  function subscribeScan(): () => void {
    return services.events.subscribeToolScan((event) => {
      scanMessage.value = event.message ?? "";
      if (event.phase === "scanning") {
        scanning.value = true;
        return;
      }
      scanning.value = false;
      if (event.phase === "failed") error.value = event.message ?? "本机工具扫描失败";
    });
  }

  async function verify(id: string): Promise<void> {
    try {
      const tool = await services.tools.verifyTool(id);
      tools.value = tools.value.map((item) => (item.id === tool.id ? tool : item));
      app.toast(
        tool.state === "ready" ? `已验证：${tool.name} 可用` : `${tool.name} 尚未通过可用性验证`,
        tool.state === "ready" ? "success" : "blocked",
      );
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "工具验证失败", "failure");
    }
  }

  async function setPath(id: string, path: string): Promise<boolean> {
    try {
      const tool = await services.tools.setToolPath(id, path);
      tools.value = tools.value.map((item) => (item.id === tool.id ? tool : item));
      app.toast("路径已保存，尚未验证", "blocked");
      return true;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "保存失败", "failure");
      return false;
    }
  }

  async function setDefault(id: string): Promise<void> {
    const tool = tools.value.find((item) => item.id === id);
    if (!tool || tool.state !== "ready") {
      app.toast("只有已验证可用的工具才能设为默认", "blocked");
      return;
    }
    try {
      await services.tools.setDefaultTool(id);
      defaultToolId.value = id;
      app.toast("已设为默认执行工具", "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "默认工具设置失败", "failure");
    }
  }

  return {
    tools,
    loading,
    scanning,
    scanMessage,
    defaultToolId,
    counts,
    error,
    mode: services.mode,
    sourceLabel: services.sourceLabel,
    load,
    subscribeScan,
    verify,
    setPath,
    setDefault,
  };
});
