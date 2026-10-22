import { computed, onScopeDispose, ref } from "vue";
import { defineStore } from "pinia";
import type { CreateTaskInput, InstructionState, Subtask, Task, TaskFilterState, TaskMessage, TaskTimeFilter } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

export type TaskFilter = {
  state: TaskFilterState;
  workspace: string;
  team: string;
  time: TaskTimeFilter;
  query: string;
};

const taskFilterStates = new Set<TaskFilterState>([
  "all", "draft", "running", "waiting-approval", "queued", "paused", "needs-attention", "done", "failed",
]);

const emptyFilter = (): TaskFilter => ({ state: "all", workspace: "全部", team: "全部", time: "all", query: "" });

/** 任务 Store：列表、详情、筛选、暂停/停止/重试等状态迁移。 */
export const useTasksStore = defineStore("tasks", () => {
  const services = useServices();
  const app = useAppStore();

  const tasks = ref<Task[]>([]);
  const selectedId = ref<string | null>(null);
  const loading = ref(false);
  const filter = ref<TaskFilter>(emptyFilter());
  let stateEventVersion = 0;
  let activeLoads = 0;
  const latestStateEvents = new Map<string, { version: number; task: Task }>();

  const selected = computed<Task | null>(() => tasks.value.find((task) => task.id === selectedId.value) ?? null);

  const visibleTasks = computed(() => {
    const f = filter.value;
    const now = Date.now();
    const ageLimit = f.time === "7d" ? 7 * 24 * 60 * 60 * 1000 : f.time === "30d" ? 30 * 24 * 60 * 60 * 1000 : null;
    return tasks.value.filter((task) => {
      if (f.query && !`${task.name}${task.workspace}${task.runId}`.toLowerCase().includes(f.query.toLowerCase())) return false;
      if (f.state !== "all" && task.state !== f.state) return false;
      if (f.workspace !== "全部" && task.workspace !== f.workspace) return false;
      if (f.team !== "全部" && !task.team.includes(f.team.replace("团队", ""))) return false;
      if (ageLimit !== null) {
        const createdAt = Date.parse(task.createdAt);
        if (!Number.isFinite(createdAt) || createdAt > now || now - createdAt > ageLimit) return false;
      }
      return true;
    });
  });

  const counters = computed(() => ({
    running: tasks.value.filter((task) => task.state === "running").length,
    waiting: tasks.value.filter((task) => ["waiting-approval"].includes(task.state)).length,
    queued: tasks.value.filter((task) => task.state === "queued").length,
    done: tasks.value.filter((task) => task.state === "done").length,
  }));

  function upsertTask(task: Task): void {
    const index = tasks.value.findIndex((item) => item.id === task.id);
    if (index === -1) {
      tasks.value = [task, ...tasks.value];
      return;
    }
    const next = [...tasks.value];
    next[index] = task;
    tasks.value = next;
  }

  const unsubscribeTaskStates = services.events.subscribeTaskState((task) => {
    const version = ++stateEventVersion;
    latestStateEvents.set(task.id, { version, task });
    upsertTask(task);
  });
  onScopeDispose(unsubscribeTaskStates);

  async function load(): Promise<void> {
    const startVersion = stateEventVersion;
    activeLoads += 1;
    loading.value = true;
    try {
      const snapshot = await services.tasks.listTasks();
      const merged = new Map(snapshot.map((task) => [task.id, task]));
      const eventOnlyTasks: Task[] = [];
      latestStateEvents.forEach(({ version, task }) => {
        if (version <= startVersion) return;
        if (merged.has(task.id)) merged.set(task.id, task);
        else eventOnlyTasks.push(task);
      });
      tasks.value = [...eventOnlyTasks, ...merged.values()];
      if (!selectedId.value && tasks.value.length) selectedId.value = tasks.value[0].id;
    } finally {
      activeLoads -= 1;
      loading.value = activeLoads > 0;
    }
  }

  function select(id: string): void {
    selectedId.value = id;
  }

  function setStateFilter(value: string): boolean {
    if (!taskFilterStates.has(value as TaskFilterState)) return false;
    filter.value.state = value as TaskFilterState;
    return true;
  }

  function setTimeFilter(value: string): boolean {
    if (value !== "7d" && value !== "30d" && value !== "all") return false;
    filter.value.time = value as TaskTimeFilter;
    return true;
  }

  async function create(input: CreateTaskInput): Promise<Task | null> {
    try {
      const task = await services.tasks.createTask(input);
      upsertTask(task);
      selectedId.value = task.id;
      app.toast(input.mode === "custom" ? "待配置占位任务已创建 · 尚未开始执行" : "任务已创建 · 状态：排队中（尚未开始执行）", "success");
      return task;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "创建任务失败", "failure");
      return null;
    }
  }

  async function pause(id: string): Promise<void> {
    try {
      const task = await services.tasks.pauseTask(id);
      upsertTask(task);
      app.toast("已停止派发新工作；当前执行将在安全边界后暂停", "blocked");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "暂停失败", "failure");
    }
  }

  async function resume(id: string): Promise<void> {
    try {
      upsertTask(await services.tasks.resumeTask(id));
      app.toast("继续：权限、依赖与输入版本检查通过，恢复调度", "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "继续失败", "failure");
    }
  }

  async function stop(id: string): Promise<void> {
    try {
      upsertTask(await services.tasks.stopTask(id));
      app.toast("正在请求停止：已向运行时发出请求，尚未确认", "neutral");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "停止失败", "failure");
    }
  }

  async function retryFailed(id: string): Promise<void> {
    try {
      const previous = tasks.value.find((task) => task.id === id)?.runId;
      const task = await services.tasks.retryFailed(id);
      upsertTask(task);
      app.toast(`已创建新的 Run ${task.runId}；旧 Run ${previous} 的记录保持不变`, "success");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "重试失败", "failure");
    }
  }

  async function skipPartial(id: string): Promise<void> {
    try {
      upsertTask(await services.tasks.skipAndFinalizePartial(id));
      app.toast("已生成部分成果：完成项与跳过项已分组标注，整体不算成功", "blocked");
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "操作失败", "failure");
    }
  }

  function appendMessage(taskId: string, text: string): void {
    const task = tasks.value.find((item) => item.id === taskId);
    if (!task) return;
    const isSingleAgent = task.mode === "single-agent";
    const recipient = isSingleAgent ? task.team.replace(/ · 单 Agent$/, "") : task.supervisor;
    const message: TaskMessage = { id: `local_${Date.now()}`, author: "user", authorName: "你", time: "现在", text };
    task.messages = [...task.messages, message];
    app.toast(isSingleAgent ? "消息已发送给 Agent" : "消息已发送给主管", "success");
    window.setTimeout(() => {
      const current = tasks.value.find((item) => item.id === taskId);
      if (!current) return;
      current.messages = [
        ...current.messages,
        {
          id: `local_${Date.now()}_reply`,
          author: "supervisor",
          authorName: isSingleAgent ? recipient : `${current.supervisor} · 主管`,
          time: "现在",
          text: "收到。我会按“来源可追溯、缺数据留空”的规则继续，需要授权时会先停下询问。",
        },
      ];
    }, 1000);
  }

  function setInstruction(taskId: string, target: string, text: string): void {
    const task = tasks.value.find((item) => item.id === taskId);
    if (!task) return;
    task.instruction = { target, text, state: "queued", impact: "该指令可能改变下一步计划：建议先做变更影响预览" };
    const advance = (state: InstructionState, delay: number, message: string, tone: "neutral" | "success" | "blocked") =>
      window.setTimeout(() => {
        if (!task.instruction) return;
        task.instruction.state = state;
        app.toast(message, tone);
      }, delay);
    advance("delivered", 1100, "已送达：运行时确认收到（尚不代表已应用）", "neutral");
    advance("waiting-point", 2300, "CLI 不支持执行中增量输入，等待安全投递点", "blocked");
    advance("applied", 3600, "已应用：下一阶段生效", "success");
    app.toast("定向指令已提交 · 排队待投递（将同步主管）", "neutral");
  }

  function resetFilter(): void {
    filter.value = emptyFilter();
  }

  function subtask(id: string): Subtask | undefined {
    return selected.value?.subtasks.find((item) => item.id === id);
  }

  return {
    tasks,
    selectedId,
    selected,
    loading,
    filter,
    visibleTasks,
    counters,
    load,
    select,
    create,
    pause,
    resume,
    stop,
    retryFailed,
    skipPartial,
    appendMessage,
    setInstruction,
    resetFilter,
    setStateFilter,
    setTimeFilter,
    subtask,
  };
});
