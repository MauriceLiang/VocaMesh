import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

const api = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke, isTauri: api.isTauri }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import CreateTaskDialog from "@/features/tasks/CreateTaskDialog.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import { __setServicesForTesting, createServices } from "@/services/adapters";

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
          return {
            ready: false,
            version: "0.162.1",
            authState: "unsupported",
            message: "Codex 的配置、凭据、文件、进程和网络边界尚未证明；真实执行已禁用",
            reasonCode: "execution.codex-safety-unverified",
          };
        }
        return {
          ready: false,
          version: null,
          authState: "unknown",
          message: "无法确认组织托管 Hook 是否会产生额外副作用；真实执行已禁用",
          reasonCode: "execution.managed-policy-unverified",
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

    resolveCodex?.({
      ready: false,
      version: "0.162.1",
      authState: "unsupported",
      message: "Codex 的安全边界尚未证明",
      reasonCode: "execution.codex-safety-unverified",
    });
    await flushPromises();
    resolveClaude?.({
      ready: true,
      version: "2.1.296",
      authState: "authenticated",
      message: "Claude 已认证",
      reasonCode: null,
    });
    await flushPromises();

    expect(document.body.textContent).toContain("Codex 的安全边界尚未证明");
    const start = Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("开始只读任务"));
    expect(start?.disabled).toBe(true);
  });
});
