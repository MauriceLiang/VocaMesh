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
    discovered: tools.value.filter((tool) => tool.discoveryStatus === "found").length,
    authPending: tools.value.filter((tool) =>
      tool.discoveryStatus === "found"
        && tool.capabilities.authenticationProbeSupported
        && tool.authState !== "authenticated",
    ).length,
    missing: tools.value.filter((tool) => tool.discoveryStatus === "missing").length,
    executionRestricted: tools.value.filter((tool) =>
      !tool.capabilities.executionAdapterAvailable
        || !tool.runtimeReady
        || tool.authState !== "authenticated"
        || tool.versionStatus !== "known",
    ).length,
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
      const ready = tool.runtimeReady && tool.authState === "authenticated";
      app.toast(
        ready ? `已验证：${tool.name} 可用于执行` : `${tool.name} 已完成探测，但执行条件尚未满足`,
        ready ? "success" : "blocked",
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
    if (!tool || !canSetPreference(tool)) {
      app.toast("只有已发现、版本有效且具有执行适配器的工具才能设为偏好", "blocked");
      return;
    }
    try {
      await services.tools.setDefaultTool(id);
      defaultToolId.value = id;
      app.toast("已设为偏好工具；执行仍需通过 Runtime 检查", "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "默认工具设置失败", "failure");
    }
  }

  function canSetPreference(tool: LocalTool): boolean {
    return tool.capabilities.executionAdapterAvailable
      && tool.discoveryStatus === "found"
      && tool.versionStatus === "known";
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
    canSetPreference,
  };
});
