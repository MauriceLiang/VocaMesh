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

  it("uses native catalog IPC and real empty states in Tauri mode", async () => {
    api.isTauri.mockReturnValue(true);
    const nativeSnapshot = [
      {
        id: "codex",
        name: "Codex CLI",
        adapter: "codex-cli",
        version: "0.162.1",
        path: "/local/codex",
        canonicalPath: "/local/codex",
        state: "undetectable",
        note: "认证探测不支持",
        authState: "unsupported",
        discoveryStatus: "found",
        versionStatus: "known",
        integrationStatus: "discovery-only",
        reasonCode: "tool.adapter-unavailable",
        runtimeReady: false,
        runtimeReasonCode: "tool.adapter-unavailable",
        capabilities: {
          providerId: "codex", displayName: "Codex CLI", executableName: "codex", adapter: "codex-cli",
          discoverySupported: true, versionProbeSupported: true, executionAdapterAvailable: false,
          authenticationProbeSupported: false, minimumVersion: null, unsupportedReasonCode: "tool.adapter-unavailable",
        },
        lastCheckedAt: 1,
        source: "path",
      },
    ];
    const services = createServices();

    expect(services.mode).toBe("tauri");
    expect(services.sourceLabel).not.toContain("演示");
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "list_agents" || command === "list_teams" || command === "list_workspaces") return [];
      if (command === "scan_tools") return nativeSnapshot;
      return null;
    });
    expect(await services.agents.listAgents()).toEqual([]);
    expect(await services.teams.listTeams()).toEqual([]);
    expect(await services.workspaces.listWorkspaces()).toEqual([]);
    expect(await services.approvals.listApprovals()).toEqual([]);
    expect(await services.notifications.listNotifications()).toEqual([]);
    expect(await services.notifications.unreadCount()).toBe(0);
    await expect(services.approvals.canGrantTaskScope()).resolves.toBe(false);
    await expect(services.approvals.decide("approval_1", "allow-once")).rejects.toMatchObject({ code: "feature.not-implemented" });
    await expect(services.notifications.markAllRead()).rejects.toMatchObject({ code: "feature.not-implemented" });
    expect(api.invoke).toHaveBeenCalledWith("list_agents", undefined);
    expect(api.invoke).toHaveBeenCalledWith("list_teams", undefined);
    expect(api.invoke).toHaveBeenCalledWith("list_workspaces", undefined);

    api.invoke.mockReset();
    api.invoke.mockRejectedValueOnce({ code: "storage.read-failed", message: "本机配置不可用" });
    await expect(services.agents.listAgents()).rejects.toMatchObject({ code: "storage.read-failed", message: "本机配置不可用" });
    expect(api.invoke).toHaveBeenCalledTimes(1);
    expect(api.invoke).toHaveBeenCalledWith("list_agents", undefined);

    api.invoke.mockReset();
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

  it("persists catalog records without requiring CLI discovery and carries revisions", async () => {
    const services = createServices("tauri");
    const agent = {
      id: "agent_1", name: "Reviewer", role: "只读分析", providerId: "codex",
      description: "", instructions: "", permissions: { read: true, write: false, shell: false, network: false }, revision: 1,
    };
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "create_agent") return agent;
      if (command === "list_agents") return [agent];
      if (command === "set_agent_permissions") return { ...agent, permissions: { ...agent.permissions, read: false }, revision: 2 };
      if (command === "update_team") return { id: "team_1", name: "Renamed", supervisorId: "agent_1", workspaceId: "workspace_1", workspace: "Local", members: [], revision: 4 };
      if (command === "get_workspace") return { id: "workspace_1", name: "Local", path: "/tmp/project", pathAvailable: true, taskCount: 0, scopes: { read: ["."], write: [], deny: [] }, revision: 8 };
      if (command === "rename_workspace") return { id: "workspace_1", name: "Renamed local", path: "/tmp/project", pathAvailable: true, taskCount: 0, scopes: { read: ["."], write: [], deny: [] }, revision: 9 };
      if (command === "update_workspace_scopes") return { id: "workspace_1", name: "Local", path: "/tmp/project", pathAvailable: true, taskCount: 0, scopes: { read: ["src"], write: [], deny: [] }, revision: 9 };
      return undefined;
    });

    const savedAgent = await services.agents.createAgent({ name: "Reviewer", role: "只读分析", toolId: "codex" });
    expect(savedAgent.state).toBe("not-ready");
    expect(savedAgent.runtimeReason).toBe("execution.managed-policy-unverified");
    expect(api.invoke).toHaveBeenCalledTimes(1);
    expect(api.invoke).toHaveBeenCalledWith("create_agent", { input: {
      name: "Reviewer", role: "只读分析", providerId: "codex", description: "", instructions: "",
      permissions: { read: true, write: false, shell: false, network: false },
    } });

    await services.agents.setPermissions("agent_1", { read: false });
    expect(api.invoke).toHaveBeenNthCalledWith(3, "set_agent_permissions", {
      id: "agent_1", permissions: { read: false, write: false, shell: false, network: false }, expectedRevision: 1,
    });

    await services.teams.updateTeam("team_1", { name: "Renamed", expectedRevision: 3 });
    expect(api.invoke).toHaveBeenLastCalledWith("update_team", { id: "team_1", input: {
      name: "Renamed", supervisorId: undefined, workspaceId: undefined, expectedRevision: 3,
    } });

    await services.workspaces.renameWorkspace("workspace_1", "Renamed local");
    expect(api.invoke).toHaveBeenNthCalledWith(5, "get_workspace", { id: "workspace_1" });
    expect(api.invoke).toHaveBeenNthCalledWith(6, "rename_workspace", {
      id: "workspace_1", name: "Renamed local", expectedRevision: 8,
    });

    await services.workspaces.updateBoundaries("workspace_1", { read: ["src"], write: [], expectedRevision: 7 });
    expect(api.invoke).toHaveBeenLastCalledWith("update_workspace_scopes", {
      id: "workspace_1", input: { scopes: { read: ["src"], write: [], deny: [] }, expectedRevision: 7 },
    });

    api.invoke.mockClear();
    await services.workspaces.renameWorkspace("workspace_1", "stale draft", 6);
    expect(api.invoke).toHaveBeenCalledOnce();
    expect(api.invoke).toHaveBeenCalledWith("rename_workspace", {
      id: "workspace_1", name: "stale draft", expectedRevision: 6,
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

    const nativeTask = {
      id: "task_1", title: "Native task", prompt: "fixture", providerId: "claude",
      workspacePath: "/tmp/project", state: "unknown", createdAt: 1, updatedAt: 2,
      currentRunId: "run_1", authorizationScope: "read-only-filtered-snapshot", consentAt: 1,
    };
    api.invoke.mockReset();
    api.invoke.mockResolvedValueOnce([nativeTask]);
    const [mappedTask] = await services.tasks.listTasks();
    expect(mappedTask.nativeExecution).toBe(true);
    expect(mappedTask.state).toBe("unknown");
    expect(mappedTask.subtasks).toEqual([]);

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
