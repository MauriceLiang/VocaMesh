import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { Agent, AgentPermissions, CreateAgentInput, UpdateAgentInput } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

/** Agent 角色 Store：名称/职责/工具/权限。与团队、任务通过 Service 保持一致。 */
export const useAgentsStore = defineStore("agents", () => {
  const services = useServices();
  const app = useAppStore();

  const agents = ref<Agent[]>([]);
  const selectedId = ref<string | null>(null);
  const loading = ref(false);
  const lastError = ref<string | null>(null);

  const selected = computed<Agent | null>(() => agents.value.find((agent) => agent.id === selectedId.value) ?? null);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      agents.value = await services.agents.listAgents();
      lastError.value = null;
      if (!selectedId.value && agents.value.length) selectedId.value = agents.value[0].id;
    } catch (err) {
      lastError.value = err instanceof ServiceError ? err.message : "无法读取 Agent 配置，请重试。";
      app.toast(lastError.value, "failure");
    } finally {
      loading.value = false;
    }
  }

  function select(id: string): void {
    selectedId.value = id;
  }

  async function create(input: CreateAgentInput): Promise<Agent | null> {
    try {
      const agent = await services.agents.createAgent(input);
      agents.value = [...agents.value, agent];
      selectedId.value = agent.id;
      app.toast(services.mode === "tauri" ? "已保存 " + agent.name + "；原生执行暂不可用" : "已创建 " + agent.name + "（状态：" + (agent.state === "ready" ? "可用" : "未就绪") + "）", "success");
      return agent;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "创建失败", "failure");
      return null;
    }
  }

  async function update(id: string, input: UpdateAgentInput): Promise<void> {
    try {
      const current = agents.value.find((item) => item.id === id);
      const agent = await services.agents.updateAgent(id, {
        ...input,
        expectedRevision: input.expectedRevision ?? current?.revision,
      });
      agents.value = agents.value.map((item) => (item.id === id ? agent : item));
      app.toast(services.mode === "tauri" ? "配置已保存；Runtime 权限未启用" : "配置已保存（从下一个新任务生效）", "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "保存失败", "failure");
    }
  }

  async function togglePermission(id: string, key: keyof AgentPermissions, value: boolean): Promise<void> {
    try {
      const agent = await services.agents.setPermissions(id, { [key]: value } as Partial<AgentPermissions>);
      agents.value = agents.value.map((item) => (item.id === id ? agent : item));
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "权限草稿保存失败", "failure");
    }
  }

  async function remove(id: string): Promise<boolean> {
    try {
      await services.agents.deleteAgent(id);
      agents.value = agents.value.filter((item) => item.id !== id);
      if (selectedId.value === id) selectedId.value = agents.value[0]?.id ?? null;
      app.toast("已归档角色（历史 Run 仍可追溯）", "blocked");
      return true;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "删除失败", "failure");
      return false;
    }
  }

  return { agents, selectedId, selected, loading, lastError, load, select, create, update, togglePermission, remove };
});
