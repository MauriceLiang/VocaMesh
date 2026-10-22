import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

const api = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke, isTauri: api.isTauri }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import CreateTaskDialog from "@/features/tasks/CreateTaskDialog.vue";
import { __setServicesForTesting, createServices } from "@/services/adapters";

describe("native task readiness", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  beforeEach(() => {
    vi.clearAllMocks();
    setActivePinia(createPinia());
    api.isTauri.mockReturnValue(true);
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "get_execution_readiness") {
        return {
          ready: false,
          version: null,
          authState: "unknown",
          message: "无法确认组织托管 Hook 是否会产生额外副作用；真实执行已禁用",
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
});
