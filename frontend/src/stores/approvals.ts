import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { ApprovalRequest } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";
import { useTasksStore } from "./tasks";

/** 审批 Store：待处理队列与决定。任务内审批卡与本页共享同一条记录。 */
export const useApprovalsStore = defineStore("approvals", () => {
  const services = useServices();
  const app = useAppStore();
  const tasks = useTasksStore();

  const approvals = ref<ApprovalRequest[]>([]);
  const selectedId = ref<string | null>(null);
  const filter = ref<"pending" | "all">("pending");
  const canGrantTaskScope = ref(false);
  const loading = ref(false);

  const pending = computed(() => approvals.value.filter((item) => item.state === "pending"));
  const selected = computed<ApprovalRequest | null>(() => approvals.value.find((item) => item.id === selectedId.value) ?? pending.value[0] ?? null);
  const visible = computed(() => (filter.value === "pending" ? pending.value : approvals.value));

  async function load(): Promise<void> {
    loading.value = true;
    try {
      approvals.value = await services.approvals.listApprovals();
      canGrantTaskScope.value = await services.approvals.canGrantTaskScope();
      if (!selectedId.value && approvals.value.length) selectedId.value = approvals.value[0].id;
    } finally {
      loading.value = false;
    }
  }

  function select(id: string): void {
    selectedId.value = id;
  }

  async function decide(id: string, decision: "allow-once" | "deny"): Promise<void> {
    try {
      const approval = await services.approvals.decide(id, decision);
      approvals.value = approvals.value.map((item) => (item.id === approval.id ? approval : item));
      await tasks.load();
      app.toast(
        decision === "allow-once" ? "已记录决定：仅此一次允许" : "已拒绝该请求 · 任务停在当前步骤，不会自动重试",
        decision === "allow-once" ? "success" : "blocked",
      );
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "提交决定失败", "failure");
    }
  }

  return { approvals, selectedId, selected, pending, visible, filter, canGrantTaskScope, loading, load, select, decide };
});
