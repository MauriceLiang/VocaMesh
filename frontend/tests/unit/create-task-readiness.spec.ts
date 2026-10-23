import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

const api = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke, isTauri: api.isTauri }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import CreateTaskDialog from "@/features/tasks/CreateTaskDialog.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import { __setServicesForTesting, createServices } from "@/services/adapters";
import type { ExecutionReadiness } from "@/types";

function readinessFixture(
  provider: "claude" | "codex",
  message: string,
  reasonCode: string | null,
  ready = false,
): ExecutionReadiness {
  const authState = provider === "codex" ? "unsupported" : ready ? "authenticated" : "unknown";
  const policyReasonCode = ready
    ? null
    : provider === "codex"
      ? "execution.codex-safety-unverified"
      : "execution.managed-policy-unverified";
  return {
    ready,
    version: provider === "codex" ? "0.162.1" : "2.1.296",
    authState,
    message,
    reasonCode,
    policyStatus: ready ? "verified" : "unverified",
    checks: [
      { id: "localStorage", status: "passed", reasonCode: null },
      { id: "activeRun", status: "passed", reasonCode: null },
      { id: "processState", status: "passed", reasonCode: null },
      { id: "platform", status: "passed", reasonCode: null },
      { id: "cliDiscovery", status: "passed", reasonCode: null },
      { id: "cliVersion", status: "passed", reasonCode: null },
      { id: "authentication", status: authState === "unsupported" ? "unsupported" : ready ? "passed" : "unknown", reasonCode: null },
      { id: "managedPolicy", status: ready ? "passed" : "blocked", reasonCode: policyReasonCode },
      { id: "taskStart", status: ready ? "passed" : "blocked", reasonCode: ready ? null : reasonCode },
    ],
    blockers: ready || !policyReasonCode
      ? []
      : [{ check: "managedPolicy", reasonCode: policyReasonCode }],
  };
}

describe("native task readiness", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  beforeEach(() => {
    vi.clearAllMocks();
    setActivePinia(createPinia());
    api.isTauri.mockReturnValue(true);
    api.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "scan_tools") return [];
      if (command === "get_default_tool_id") return null;
      if (command === "get_execution_readiness") {
        if (args?.providerId === "codex") {
          return readinessFixture(
            "codex",
            "Codex 的配置、凭据、文件、进程和网络边界尚未证明；真实执行已禁用",
            "execution.codex-safety-unverified",
          );
        }
        return {
          ready: false,
          version: null,
          authState: "authenticated",
          message: "无法确认组织托管 Hook 是否会产生额外副作用；真实执行已禁用",
          reasonCode: "execution.managed-policy-unverified",
          policyStatus: "unverified",
          checks: [
            { id: "localStorage", status: "passed", reasonCode: null },
            { id: "activeRun", status: "passed", reasonCode: null },
            { id: "processState", status: "passed", reasonCode: null },
            { id: "platform", status: "passed", reasonCode: null },
            { id: "cliDiscovery", status: "passed", reasonCode: null },
            { id: "cliVersion", status: "passed", reasonCode: null },
            { id: "authentication", status: "passed", reasonCode: null },
            { id: "managedPolicy", status: "blocked", reasonCode: "execution.managed-policy-unverified" },
            { id: "taskStart", status: "blocked", reasonCode: "execution.managed-policy-unverified" },
          ],
          blockers: [{ check: "managedPolicy", reasonCode: "execution.managed-policy-unverified" }],
        };
      }
      return undefined;
    });
    api.listen.mockResolvedValue(() => undefined);
    __setServicesForTesting(createServices("tauri"));
  });

  it("keeps start disabled and does not invoke task start when readiness is blocked", async () => {
    const wrapper = mount(CreateTaskDialog, {
      props: { modelValue: false },
      global: { plugins: [createPinia()] },
      attachTo: document.body,
    });

    await wrapper.setProps({ modelValue: true });
    await flushPromises();

    expect(document.body.textContent).toContain("无法确认组织托管 Hook");
    expect(document.body.textContent).toContain("Claude Code 已登录，但执行仍被托管策略阻断。");
    expect(document.body.textContent).toContain("托管策略");
    expect(document.body.textContent).toContain("Provider 认证");
    const start = Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("开始只读任务"));
    expect(start?.disabled).toBe(true);
    start?.click();
    await flushPromises();
    expect(api.invoke).not.toHaveBeenCalledWith(
      "start_read_only_task",
      expect.anything(),
    );
  });

  it("loads Codex-specific readiness and keeps its blocked task from starting", async () => {
    const wrapper = mount(CreateTaskDialog, {
      props: { modelValue: false },
      global: { plugins: [createPinia()] },
      attachTo: document.body,
    });

    await wrapper.setProps({ modelValue: true });
    await flushPromises();
    wrapper.findComponent(VocaSelect).vm.$emit("update:modelValue", "codex");
    await flushPromises();

    expect(api.invoke).toHaveBeenCalledWith("get_execution_readiness", { providerId: "codex" });
    expect(document.body.textContent).toContain("Codex 的配置、凭据、文件、进程和网络边界尚未证明");
    const start = Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("开始只读任务"));
    expect(start?.disabled).toBe(true);
    start?.click();
    await flushPromises();
    expect(api.invoke).not.toHaveBeenCalledWith(
      "start_read_only_task",
      expect.objectContaining({ request: expect.objectContaining({ providerId: "codex" }) }),
    );
  });

  it("does not apply an older readiness result after changing providers", async () => {
    let resolveClaude: ((value: unknown) => void) | undefined;
    let resolveCodex: ((value: unknown) => void) | undefined;
    api.invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === "scan_tools") return Promise.resolve([]);
      if (command === "get_default_tool_id") return Promise.resolve(null);
      if (command === "get_execution_readiness") {
        return new Promise<unknown>((resolve) => {
          if (args?.providerId === "codex") resolveCodex = resolve;
          else resolveClaude = resolve;
        });
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(CreateTaskDialog, {
      props: { modelValue: false },
      global: { plugins: [createPinia()] },
      attachTo: document.body,
    });
    await wrapper.setProps({ modelValue: true });
    await flushPromises();
    wrapper.findComponent(VocaSelect).vm.$emit("update:modelValue", "codex");
    await flushPromises();

    resolveCodex?.(readinessFixture("codex", "Codex 的安全边界尚未证明", "execution.codex-safety-unverified"));
    await flushPromises();
    resolveClaude?.(readinessFixture("claude", "Claude 已认证", null, true));
    await flushPromises();

    expect(document.body.textContent).toContain("Codex 的安全边界尚未证明");
    const start = Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("开始只读任务"));
    expect(start?.disabled).toBe(true);
  });
});
