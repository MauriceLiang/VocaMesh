import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";

const api = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke, isTauri: api.isTauri }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import RuntimeRecoveryPanel from "@/features/tasks/RuntimeRecoveryPanel.vue";
import { __setServicesForTesting, createServices } from "@/services/adapters";

const run = {
  id: "run_0123456789abcdef0123456789abcdef",
  requestId: "00000000-0000-4000-8000-000000000201",
  taskId: "task_1",
  providerId: "claude",
  state: "unknown",
  outcome: "unknown",
  startedAt: 1000,
  finishedAt: 2000,
  exitCode: null,
  summary: null,
  lastSequence: 1,
  outputTruncated: false,
  processId: 42,
  errorCode: "run.stop-unconfirmed",
  recoveryStatus: "required",
  recoveryCheckedAt: null,
};
const summary = { run, taskTitle: "Synthetic recovery", snapshotStatus: "exists" };
const inspection = {
  run: { ...run, recoveryCheckedAt: 3000 },
  taskTitle: "Synthetic recovery",
  processGroupStatus: "manual-required",
  snapshotStatus: "exists",
};

describe("runtime recovery panel", () => {
  afterEach(() => {
    document.body.innerHTML = "";
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    api.isTauri.mockReturnValue(true);
    api.listen.mockResolvedValue(() => undefined);
    __setServicesForTesting(createServices("tauri"));
  });

  it("keeps unknown-run data visible and does not offer archive for manual-required", async () => {
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "list_unknown_runs") return [summary];
      if (command === "inspect_unknown_run") return inspection;
      return undefined;
    });
    const wrapper = mount(RuntimeRecoveryPanel, { attachTo: document.body });
    await flushPromises();

    expect(document.body.textContent).toContain("Synthetic recovery");
    expect(document.body.textContent).toContain("快照：存在");
    const buttons = Array.from(document.body.querySelectorAll("button"));
    buttons.find((button) => button.textContent?.includes("重新检查"))?.click();
    await flushPromises();

    expect(document.body.textContent).toContain("需要人工处理");
    expect(document.body.textContent).not.toContain("我已人工核验并归档");
    expect(api.invoke).not.toHaveBeenCalledWith("resolve_unknown_run", expect.anything());
    wrapper.unmount();
  });

  it("requires a second human confirmation before resolving an exited candidate", async () => {
    let listCalls = 0;
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "list_unknown_runs") return listCalls++ === 0 ? [summary] : [];
      if (command === "inspect_unknown_run") {
        return { ...inspection, processGroupStatus: "exited-candidate" };
      }
      if (command === "resolve_unknown_run") return { run: { ...run, state: "crashed", outcome: "recovered-after-exit" }, snapshotStatus: "missing" };
      return undefined;
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const wrapper = mount(RuntimeRecoveryPanel, { attachTo: document.body });
    await flushPromises();
    Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("重新检查"))?.click();
    await flushPromises();

    expect(document.body.textContent).toContain("仅为人工归档候选");
    Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("我已人工核验并归档"))?.click();
    await flushPromises();

    expect(window.confirm).toHaveBeenCalledOnce();
    expect(api.invoke).toHaveBeenCalledWith("resolve_unknown_run", { runId: run.id, userConfirmed: true });
    expect(document.body.textContent).not.toContain("Synthetic recovery");
    expect(document.body.textContent).toContain("Run 已归档为失败，快照已清理");
    wrapper.unmount();
  });

  it("retains the listed run and shows an IPC error when inspection fails", async () => {
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "list_unknown_runs") return [summary];
      if (command === "inspect_unknown_run") throw { code: "storage.read-failed", message: "read failed" };
      return undefined;
    });
    const wrapper = mount(RuntimeRecoveryPanel, { attachTo: document.body });
    await flushPromises();
    Array.from(document.body.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("重新检查"))?.click();
    await flushPromises();

    expect(document.body.textContent).toContain("Synthetic recovery");
    expect(document.body.textContent).toContain("检查失败");
    expect(document.body.textContent).not.toContain("read failed");
    wrapper.unmount();
  });
});
