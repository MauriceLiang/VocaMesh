import { describe, expect, it } from "vitest";
import { approvalStateMeta, runStateMeta, subtaskStateMeta, taskStateMeta, toolStateMeta } from "@/lib/status";

/** 状态语义测试：确保同一状态在任何页面得到同一 tone/label（视觉一致性）。 */
describe("status tokens", () => {
  it("成功/失败/阻塞使用固定语义色", () => {
    expect(taskStateMeta("done").tone).toBe("success");
    expect(taskStateMeta("failed").tone).toBe("failure");
    expect(taskStateMeta("waiting-approval").tone).toBe("blocked");
    expect(taskStateMeta("partial").tone).toBe("blocked");
    expect(subtaskStateMeta("blocked").tone).toBe("blocked");
    expect(toolStateMeta("ready").tone).toBe("success");
    expect(toolStateMeta("undetectable").tone).toBe("unknown");
    expect(runStateMeta("crashed").tone).toBe("failure");
    expect(approvalStateMeta("denied").tone).toBe("failure");
  });

  it("运行中与暂停属于中性色，不占用语义色", () => {
    expect(taskStateMeta("running").tone).toBe("neutral");
    expect(taskStateMeta("paused").tone).toBe("neutral");
    expect(taskStateMeta("pause-pending").tone).toBe("neutral");
    expect(taskStateMeta("stop-pending").tone).toBe("unknown");
  });
});
