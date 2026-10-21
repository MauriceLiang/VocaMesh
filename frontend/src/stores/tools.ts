import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { LocalTool } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

/**
 * 本地工具 Store：回答“这台电脑上有什么可运行的环境”。
 * 与 Agent 角色严格分离；状态不明时显示“无法检测”，不假定可用。
 */
export const useToolsStore = defineStore("tools", () => {
  const services = useServices();
  const app = useAppStore();

  const tools = ref<LocalTool[]>([]);
  const loading = ref(false);
  const scanning = ref(false);
  const defaultToolId = ref<string>("claude");
  const error = ref<string | null>(null);

  const counts = computed(() => ({
    ready: tools.value.filter((tool) => tool.state === "ready").length,
    authRequired: tools.value.filter((tool) => tool.state === "auth-required").length,
    missing: tools.value.filter((tool) => tool.state === "missing").length,
    undetectable: tools.value.filter((tool) => tool.state === "undetectable").length,
  }));

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      tools.value = await services.tools.scanTools();
      defaultToolId.value = await services.tools.getDefaultToolId();
    } catch (err) {
      error.value = err instanceof ServiceError ? err.message : "工具检测失败";
      app.toast(error.value, "failure");
    } finally {
      loading.value = false;
    }
  }

  async function verify(id: string): Promise<void> {
    const tool = await services.tools.verifyTool(id);
    tools.value = tools.value.map((item) => (item.id === tool.id ? tool : item));
    app.toast(
      tool.state === "ready" ? `已验证：${tool.name} 可用` : `${tool.name} 仍无法确认状态`,
      tool.state === "ready" ? "success" : "blocked",
    );
  }

  async function setPath(id: string, path: string): Promise<void> {
    try {
      const tool = await services.tools.setToolPath(id, path);
      tools.value = tools.value.map((item) => (item.id === tool.id ? tool : item));
      app.toast("路径已保存，状态为“无法检测”，请再次验证", "blocked");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "保存失败", "failure");
    }
  }

  async function setDefault(id: string): Promise<void> {
    await services.tools.setDefaultTool(id);
    defaultToolId.value = id;
    app.toast("已设为默认执行工具", "success");
  }

  return { tools, loading, scanning, defaultToolId, counts, error, load, verify, setPath, setDefault };
});
