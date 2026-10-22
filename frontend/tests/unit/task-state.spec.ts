import { describe, expect, it } from "vitest";
import { approvalStateMeta, runStateMeta, subtaskStateMeta, taskBannerTone, taskStateMeta, toolStateMeta } from "@/lib/status";

/** 状态语义测试：确保同一状态在任何页面得到同一 tone/label（视觉一致性）。 */
describe("status tokens", () => {
  it("成功/失败/阻塞使用固定语义色", () => {
    expect(taskStateMeta("done").tone).toBe("success");
    expect(taskStateMeta("failed")).toMatchObject({ label: "执行失败", tone: "failure" });
    expect(taskBannerTone("failed")).toBe("is-failure");
    expect(taskStateMeta("needs-attention")).toMatchObject({ label: "需要处理", tone: "blocked" });
    expect(taskBannerTone("needs-attention")).toBe("is-blocked");
    expect(taskStateMeta("partial")).toMatchObject({ label: "部分完成", tone: "blocked" });
    expect(taskBannerTone("partial")).toBe("is-blocked");
    expect(taskStateMeta("waiting-approval").tone).toBe("blocked");
    expect(subtaskStateMeta("blocked").tone).toBe("blocked");
    expect(toolStateMeta("ready").tone).toBe("success");
    expect(toolStateMeta("undetectable").tone).toBe("unknown");
    expect(runStateMeta("crashed").tone).toBe("failure");
    expect(approvalStateMeta("denied").tone).toBe("failure");
  });

  it("运行中与暂停属于中性色，不占用语义色", () => {
    expect(taskStateMeta("draft")).toMatchObject({ label: "待配置", tone: "neutral" });
    expect(taskStateMeta("running").tone).toBe("neutral");
    expect(taskStateMeta("paused").tone).toBe("neutral");
    expect(taskStateMeta("pause-pending").tone).toBe("neutral");
    expect(taskStateMeta("stop-pending").tone).toBe("unknown");
  });
});
