import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, disposePinia, setActivePinia } from "pinia";
import { __setServicesForTesting, createServices } from "@/services/adapters";
import type { Task } from "@/types";
import { taskStateFilterOptions } from "@/lib/task-filters";
import { useTasksStore } from "@/stores/tasks";

const day = 24 * 60 * 60 * 1000;

function taskFixture(id: string, patch: Partial<Task> = {}): Task {
  return {
    id,
    name: `${id} task`,
    workspace: "research-2025",
    mode: "team",
    team: "调研团队 · 4 人",
    runId: `#${id}`,
    supervisor: "研究协调员",
    createdAt: new Date("2026-10-22T00:00:00Z").toISOString(),
    startedAt: "现在",
    state: "running",
    stateNote: "fixture",
    subtasks: [],
    messages: [],
    instruction: null,
    ...patch,
  };
}

describe("tasks store", () => {
  let pinia: ReturnType<typeof createPinia>;
  let services: ReturnType<typeof createServices>;

  beforeEach(() => {
    services = createServices();
    __setServicesForTesting(services);
    pinia = createPinia();
    setActivePinia(pinia);
  });

  afterEach(() => {
    disposePinia(pinia);
    __setServicesForTesting(null);
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("subscribes once and applies create, run, pause, resume, and stop state events", async () => {
    vi.useFakeTimers();
    const store = useTasksStore();
    await store.load();

    await store.pause("t1");
    expect(store.tasks.find((task) => task.id === "t1")?.state).toBe("pause-pending");
    await vi.advanceTimersByTimeAsync(1700);
    expect(store.tasks.find((task) => task.id === "t1")?.state).toBe("paused");

    await store.resume("t1");
    expect(store.tasks.find((task) => task.id === "t1")?.state).toBe("running");
    await store.stop("t1");
    expect(store.tasks.find((task) => task.id === "t1")?.state).toBe("stop-pending");
    await vi.advanceTimersByTimeAsync(1800);
    expect(store.tasks.find((task) => task.id === "t1")?.state).toBe("stopped");

    const created = await store.create({ description: "新任务", workspace: "research-2025", mode: "team", teamId: "tm1" });
    expect(created?.state).toBe("queued");
    expect(store.tasks.filter((task) => task.id === created?.id)).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(1400);
    expect(store.tasks.find((task) => task.id === created?.id)?.state).toBe("running");
    expect(store.counters.running).toBe(2);
    expect(store.visibleTasks.find((task) => task.id === created?.id)?.state).toBe("running");
  });

  it("cleans up the single global state subscription when the Store scope ends", () => {
    const subscribe = services.events.subscribeTaskState.bind(services.events);
    const unsubscribeSpy = vi.fn();
    const subscriptionSpy = vi.spyOn(services.events, "subscribeTaskState").mockImplementation((handler) => {
      const unsubscribe = subscribe(handler);
      return () => {
        unsubscribe();
        unsubscribeSpy();
      };
    });

    const store = useTasksStore();
    expect(useTasksStore()).toBe(store);
    expect(subscriptionSpy).toHaveBeenCalledTimes(1);
    disposePinia(pinia);
    expect(unsubscribeSpy).toHaveBeenCalledTimes(1);
  });

  it("keeps a state event received during load instead of restoring the stale list snapshot", async () => {
    vi.useFakeTimers();
    const staleSnapshot = await services.tasks.listTasks();
    let resolveSnapshot!: (tasks: Task[]) => void;
    vi.spyOn(services.tasks, "listTasks").mockImplementation(() => new Promise((resolve) => { resolveSnapshot = resolve; }));
    const store = useTasksStore();
    const loading = store.load();

    const pending = await services.tasks.pauseTask("t1");
    const created = await services.tasks.createTask({ description: "竞态任务", workspace: "research-2025", mode: "single-agent", agentId: "r2" });
    resolveSnapshot(staleSnapshot);
    await loading;

    expect(store.tasks.find((task) => task.id === "t1")?.state).toBe(pending.state);
    expect(store.tasks.filter((task) => task.id === created.id)).toHaveLength(1);
    expect(store.selected?.id).toBe(created.id);
  });

  it("maps every displayed Chinese state filter to its domain code and rejects unknown values", () => {
    const states = taskStateFilterOptions.filter((option) => option.value !== "all");
    const fixtures = states.map((option) => taskFixture(option.value, {
      mode: option.value === "draft" ? "custom" : "team",
      state: option.value as Task["state"],
    }));
    const store = useTasksStore();
    store.tasks = fixtures;

    for (const option of states) {
      expect(store.setStateFilter(option.value)).toBe(true);
      expect(store.visibleTasks.map((task) => task.state)).toEqual([option.value]);
    }
    expect(store.setStateFilter("running")).toBe(true);
    expect(store.setStateFilter("执行中")).toBe(false);
    expect(store.filter.state).toBe("running");
    expect(store.setStateFilter("not-a-state")).toBe(false);
    expect(store.filter.state).toBe("running");

    expect(store.setStateFilter("all")).toBe(true);
    expect(store.visibleTasks).toHaveLength(states.length);
  });

  it("filters by creation time and combines date, state, workspace, executor, and search", () => {
    vi.useFakeTimers();
    const now = new Date("2026-10-22T00:00:00Z");
    vi.setSystemTime(now);
    const store = useTasksStore();
    store.tasks = [
      taskFixture("recent", { name: "needle report", createdAt: new Date(now.getTime() - 6 * day).toISOString() }),
      taskFixture("week-boundary", { createdAt: new Date(now.getTime() - 7 * day).toISOString() }),
      taskFixture("wrong-workspace", { name: "needle other", workspace: "vocab-ops", createdAt: new Date(now.getTime() - 2 * day).toISOString() }),
      taskFixture("wrong-executor", { name: "needle other", team: "内容团队 · 2 人", createdAt: new Date(now.getTime() - 2 * day).toISOString() }),
      taskFixture("wrong-state", { name: "needle other", state: "paused", createdAt: new Date(now.getTime() - 2 * day).toISOString() }),
      taskFixture("wrong-query", { name: "other report", createdAt: new Date(now.getTime() - 2 * day).toISOString() }),
      taskFixture("middle", { createdAt: new Date(now.getTime() - 20 * day).toISOString() }),
      taskFixture("month-boundary", { createdAt: new Date(now.getTime() - 30 * day).toISOString() }),
      taskFixture("old", { createdAt: new Date(now.getTime() - 31 * day).toISOString() }),
      taskFixture("invalid", { createdAt: "not-a-date" }),
      taskFixture("future", { createdAt: new Date(now.getTime() + day).toISOString() }),
    ];

    expect(store.setTimeFilter("7d")).toBe(true);
    expect(store.visibleTasks.map((task) => task.id)).toEqual([
      "recent", "week-boundary", "wrong-workspace", "wrong-executor", "wrong-state", "wrong-query",
    ]);
    expect(store.setTimeFilter("30d")).toBe(true);
    expect(store.visibleTasks.map((task) => task.id)).toContain("month-boundary");
    expect(store.visibleTasks.map((task) => task.id)).not.toContain("old");
    expect(store.setTimeFilter("all")).toBe(true);
    expect(store.visibleTasks).toHaveLength(11);
    expect(store.setTimeFilter("recent")).toBe(false);
    expect(store.filter.time).toBe("all");

    store.filter.state = "running";
    store.filter.workspace = "research-2025";
    store.filter.team = "调研团队";
    store.filter.time = "7d";
    store.filter.query = "needle";
    expect(store.visibleTasks.map((task) => task.id)).toEqual(["recent"]);

    store.resetFilter();
    expect(store.filter).toEqual({ state: "all", workspace: "全部", team: "全部", time: "all", query: "" });
  });

  it("shows invalid creation timestamps only when all time is selected", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-10-22T00:00:00Z"));
    const store = useTasksStore();
    store.tasks = [taskFixture("invalid", { createdAt: "" })];
    store.filter.time = "30d";
    expect(store.visibleTasks).toEqual([]);
    store.filter.time = "all";
    expect(store.visibleTasks.map((task) => task.id)).toEqual(["invalid"]);
  });
});
