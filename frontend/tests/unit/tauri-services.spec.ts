import { beforeEach, describe, expect, it, vi } from "vitest";

const api = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke, isTauri: api.isTauri }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import { createServices } from "@/services/adapters";

describe("runtime service adapter", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.isTauri.mockReturnValue(false);
  });

  it("keeps browser preview on Mock and does not call native IPC", async () => {
    const services = createServices();
    expect(services.mode).toBe("mock");
    expect(services.sourceLabel).toBe("Mock 演示数据");
    expect(await services.tools.getDefaultToolId()).toBe("claude");
    expect(api.invoke).not.toHaveBeenCalled();
  });

  it("replaces only tool discovery in Tauri mode and maps stable IPC errors", async () => {
    api.isTauri.mockReturnValue(true);
    const nativeSnapshot = [
      {
        id: "codex",
        name: "Codex CLI",
        adapter: "codex-cli",
        version: "0.162.1",
        path: "/local/codex",
        state: "undetectable",
        note: "认证探测不支持",
        authState: "unsupported",
        detectedAt: 1,
        source: "path",
      },
    ];
    const services = createServices();

    expect(services.mode).toBe("tauri");
    expect(services.sourceLabel).toContain("仍为演示数据");
    expect(await services.agents.listAgents()).toHaveLength(5);
    expect(api.invoke).not.toHaveBeenCalled();

    api.invoke.mockResolvedValueOnce(nativeSnapshot);
    expect(await services.tools.scanTools()).toEqual(nativeSnapshot);
    api.invoke.mockResolvedValueOnce(null);
    expect(await services.tools.getDefaultToolId()).toBeNull();
    expect(api.invoke).toHaveBeenCalledWith("get_default_tool_id", undefined);

    api.invoke.mockRejectedValueOnce({ code: "tool.not-ready", message: "工具尚未验证" });
    await expect(services.tools.setDefaultTool("codex")).rejects.toMatchObject({
      name: "ServiceError",
      code: "tool.not-ready",
      message: "工具尚未验证",
    });
  });

  it("cleans up a listener even when it resolves after unsubscription", async () => {
    const stop = vi.fn();
    let resolveListen!: (value: () => void) => void;
    api.listen.mockReturnValue(new Promise((resolve) => (resolveListen = resolve)));
    const services = createServices("tauri");
    const unsubscribe = services.events.subscribeToolScan(vi.fn());
    unsubscribe();
    resolveListen(stop);
    await Promise.resolve();
    expect(stop).toHaveBeenCalledOnce();
  });

  it("uses SQLite-backed Task and Run IPC in desktop mode and preserves browser Mock", async () => {
    const services = createServices("tauri");
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "list_tasks") return [];
      if (command === "list_runs") return { items: [], hasMore: false, nextCursor: null };
      return undefined;
    });

    expect(await services.tasks.listTasks()).toEqual([]);
    expect((await services.runs.listRuns()).items).toEqual([]);
    expect(api.invoke).toHaveBeenCalledWith("list_tasks", undefined);
    expect(api.invoke).toHaveBeenCalledWith("list_runs", { cursor: undefined, limit: 50 });

    api.invoke.mockClear();
    const browser = createServices("mock");
    expect(await browser.tasks.listTasks()).not.toHaveLength(0);
    expect(await browser.execution.listUnknownRuns()).toEqual([]);
    expect(api.invoke).not.toHaveBeenCalled();
  });

  it("routes recovery inspection and confirmation through Tauri execution IPC", async () => {
    const services = createServices("tauri");
    api.invoke.mockResolvedValueOnce([]);
    expect(await services.execution.listUnknownRuns()).toEqual([]);
    api.invoke.mockResolvedValueOnce({ processGroupStatus: "exited-candidate" });
    expect(await services.execution.inspectUnknownRun("run_1")).toEqual({ processGroupStatus: "exited-candidate" });
    const resolution = { run: { id: "run_1", state: "crashed" }, snapshotStatus: "missing" };
    api.invoke.mockResolvedValueOnce(resolution);
    expect(await services.execution.resolveUnknownRun("run_1", true)).toEqual(resolution);
    expect(api.invoke).toHaveBeenNthCalledWith(1, "list_unknown_runs", undefined);
    expect(api.invoke).toHaveBeenNthCalledWith(2, "inspect_unknown_run", { runId: "run_1" });
    expect(api.invoke).toHaveBeenNthCalledWith(3, "resolve_unknown_run", { runId: "run_1", userConfirmed: true });
  });

  it("passes the confirmed task request to native IPC and replays ordered, deduplicated run events", async () => {
    const services = createServices("tauri");
    const input = {
      requestId: "00000000-0000-4000-8000-000000000001",
      title: "Read-only check",
      prompt: "Summarize the folder",
      providerId: "claude" as const,
      workspacePath: "/tmp/project",
      consent: true,
    };
    const task = {
      id: "task_1", title: input.title, prompt: input.prompt, providerId: "claude",
      workspacePath: input.workspacePath, state: "queued", createdAt: 1, updatedAt: 1,
      currentRunId: "run_1", authorizationScope: "read-only-filtered-snapshot", consentAt: 1,
    };
    const run = {
      id: "run_1", requestId: input.requestId, taskId: "task_1", providerId: "claude",
      state: "pending", outcome: null, startedAt: null, finishedAt: null, exitCode: null,
      summary: null, lastSequence: 2, outputTruncated: false, processId: null, errorCode: null,
      recoveryStatus: "none", recoveryCheckedAt: null,
    };
    api.invoke.mockResolvedValueOnce({ task, run });
    await expect(services.execution.startReadOnlyTask(input)).resolves.toEqual({ task, run });
    expect(api.invoke).toHaveBeenCalledWith("start_read_only_task", { request: input });

    const first = { runId: "run_1", sequence: 1, timestamp: 1, kind: "text", text: "A" };
    const second = { runId: "run_1", sequence: 2, timestamp: 2, kind: "final", text: "AB" };
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "get_run") return run;
      if (command === "list_run_events") return [first, second];
      return undefined;
    });
    let nativeListener!: (event: { payload: typeof second }) => void;
    api.listen.mockImplementation(async (_name: string, handler: typeof nativeListener) => {
      nativeListener = handler;
      return () => undefined;
    });
    const lines: number[] = [];
    services.events.subscribeTaskLogs("run_1", (line) => lines.push(line.sequence!));
    nativeListener({ payload: second });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(lines).toEqual([1, 2]);
    nativeListener({ payload: { ...second, sequence: 3 } });
    expect(lines).toEqual([1, 2, 3]);
  });
});
