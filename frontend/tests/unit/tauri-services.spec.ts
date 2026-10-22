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
});
