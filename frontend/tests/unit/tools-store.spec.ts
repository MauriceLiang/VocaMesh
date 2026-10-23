import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { LocalTool } from "@/types";
import { __setServicesForTesting, createServices, type AppServices } from "@/services/adapters";
import { useToolsStore } from "@/stores/tools";

const tool = (id: string, state: LocalTool["state"]): LocalTool => ({
  id,
  name: id,
  adapter: id,
  version: "1.2.3",
  path: `/tools/${id}`,
  state,
  note: "test fixture",
  canonicalPath: state === "missing" ? null : `/real/tools/${id}`,
  authState: id === "claude" ? "unknown" : "unsupported",
  discoveryStatus: state === "missing" ? "missing" : "found",
  versionStatus: state === "missing" ? "unknown" : "known",
  integrationStatus: id === "claude" ? "adapter-available" : "discovery-only",
  reasonCode: id === "claude" ? null : "tool.adapter-unavailable",
  runtimeReady: false,
  runtimeReasonCode: id === "claude" ? "execution.managed-policy-unverified" : "tool.adapter-unavailable",
  capabilities: {
    providerId: id,
    displayName: id,
    executableName: id,
    adapter: id,
    discoverySupported: true,
    versionProbeSupported: true,
    executionAdapterAvailable: id === "claude",
    authenticationProbeSupported: id === "claude",
    minimumVersion: id === "claude" ? "2.1.259" : null,
    unsupportedReasonCode: id === "claude" ? null : "tool.adapter-unavailable",
  },
  lastCheckedAt: 1,
  source: state === "missing" ? null : "manual",
});

function installServices(overrides: Partial<AppServices["tools"]>): AppServices {
  const mock = createServices("mock");
  const services = {
    ...mock,
    mode: "tauri" as const,
    sourceLabel: "本机工具检测已接入；其他演示数据",
    tools: { ...mock.tools, ...overrides },
  };
  __setServicesForTesting(services);
  return services;
}

describe("tools store", () => {
  afterEach(() => {
    __setServicesForTesting(null);
    vi.restoreAllMocks();
  });

  it("keeps the newest scan snapshot when scans finish out of order", async () => {
    const resolvers: Array<(value: LocalTool[]) => void> = [];
    installServices({
      scanTools: vi.fn(() => new Promise<LocalTool[]>((resolve) => resolvers.push(resolve))),
      getDefaultToolId: vi.fn().mockResolvedValue(null),
    });
    setActivePinia(createPinia());
    const store = useToolsStore();

    const older = store.load();
    const newer = store.load();
    resolvers[1]([tool("codex", "undetectable")]);
    await newer;
    resolvers[0]([tool("claude", "missing")]);
    await older;

    expect(store.tools.map((item) => item.id)).toEqual(["codex"]);
    expect(store.loading).toBe(false);
    expect(store.scanning).toBe(false);
  });

  it("preserves the stored preference when a scan finds an unsupported provider", async () => {
    installServices({
      scanTools: vi.fn().mockResolvedValue([tool("codex", "undetectable")]),
      getDefaultToolId: vi.fn().mockResolvedValue("codex"),
    });
    setActivePinia(createPinia());
    const store = useToolsStore();

    await store.load();

    expect(store.defaultToolId).toBe("codex");
    expect(store.tools[0]?.integrationStatus).toBe("discovery-only");
    expect(store.counts.authPending).toBe(0);
  });

  it("keeps ordinary scans separate from active auth verification", async () => {
    const verifyTool = vi.fn();
    const scanned = { ...tool("claude", "pending"), reasonCode: "execution.auth-not-verified" };
    installServices({
      scanTools: vi.fn().mockResolvedValue([scanned]),
      getDefaultToolId: vi.fn().mockResolvedValue(null),
      verifyTool,
    });
    setActivePinia(createPinia());
    const store = useToolsStore();

    await store.load();

    expect(verifyTool).not.toHaveBeenCalled();
    expect(store.tools[0]?.authState).toBe("unknown");
    expect(store.tools[0]?.reasonCode).toBe("execution.auth-not-verified");
  });

  it("serializes active verification and clears stale success after an IPC failure", async () => {
    const verifyTool = vi.fn().mockRejectedValue(new Error("private IPC detail"));
    installServices({ verifyTool });
    setActivePinia(createPinia());
    const store = useToolsStore();
    store.tools = [{
      ...tool("claude", "ready"),
      authState: "authenticated",
      runtimeReady: true,
      reasonCode: null,
    }];

    const first = store.verify("claude");
    expect(store.verifyingToolId).toBe("claude");
    await store.verify("codex");
    await first;

    expect(verifyTool).toHaveBeenCalledTimes(1);
    expect(store.verifyingToolId).toBeNull();
    expect(store.tools[0]?.authState).toBe("unknown");
    expect(store.tools[0]?.reasonCode).toBe("execution.auth-probe-failed");
    expect(store.tools[0]?.runtimeReady).toBe(false);
    expect(store.error).toBe("工具验证失败");
  });

  it("allows an eligible adapter to be preferred while Runtime remains blocked", async () => {
    const setDefault = vi.fn();
    installServices({ setDefaultTool: setDefault });
    setActivePinia(createPinia());
    const store = useToolsStore();
    store.tools = [tool("claude", "undetectable")];

    await store.setDefault("claude");

    expect(setDefault).toHaveBeenCalledWith("claude");
    expect(store.defaultToolId).toBe("claude");
  });

  it("shows scan errors and refuses a non-ready default before IPC", async () => {
    const setDefault = vi.fn();
    installServices({
      scanTools: vi.fn().mockRejectedValue(new Error("private process output")),
      setDefaultTool: setDefault,
    });
    setActivePinia(createPinia());
    const store = useToolsStore();

    await store.load();
    expect(store.error).toBe("工具检测失败");
    store.tools = [tool("codex", "undetectable")];
    await store.setDefault("codex");
    expect(setDefault).not.toHaveBeenCalled();
  });

  it("keeps a saved path in the unverified state", async () => {
    const setToolPath = vi.fn().mockResolvedValue(tool("codex", "undetectable"));
    installServices({ setToolPath });
    setActivePinia(createPinia());
    const store = useToolsStore();
    store.tools = [tool("codex", "missing")];

    await store.setPath("codex", "/local/codex");
    expect(setToolPath).toHaveBeenCalledWith("codex", "/local/codex");
    expect(store.tools[0]?.state).toBe("undetectable");
  });
});
