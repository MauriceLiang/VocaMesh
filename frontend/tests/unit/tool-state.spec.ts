import { describe, expect, it } from "vitest";
import { toolStateMeta } from "@/lib/status";
import type { ToolState } from "@/types";

describe("tool status labels", () => {
  it("maps every runtime state to explicit UI text", () => {
    const states: ToolState[] = ["ready", "pending", "auth-required", "missing", "incompatible", "undetectable"];
    expect(states.map((state) => toolStateMeta(state).label)).toEqual([
      "已安装且可用",
      "待验证",
      "认证待验证",
      "未安装",
      "不兼容",
      "无法检测",
    ]);
  });
});
