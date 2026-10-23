import { describe, expect, it } from "vitest";
import {
  toolAuthMeta,
  toolDiscoveryMeta,
  toolIntegrationMeta,
  toolRuntimeMeta,
  toolStateMeta,
  toolVersionMeta,
} from "@/lib/status";
import type {
  ToolAuthState,
  ToolDiscoveryStatus,
  ToolIntegrationStatus,
  ToolState,
  ToolVersionStatus,
} from "@/types";

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

  it("maps discovery, version, auth, adapter, and Runtime states independently", () => {
    const discovery: ToolDiscoveryStatus[] = ["missing", "found", "path-invalid", "probe-failed"];
    const versions: ToolVersionStatus[] = ["known", "unknown", "incompatible"];
    const auth: ToolAuthState[] = ["authenticated", "unauthenticated", "unknown", "unsupported"];
    const integrations: ToolIntegrationStatus[] = ["discovery-only", "adapter-available", "unsupported-platform", "blocked-by-policy"];

    expect(discovery.map((state) => toolDiscoveryMeta(state).label)).toEqual(["未找到", "已发现", "路径无效", "探测失败"]);
    expect(versions.map((state) => toolVersionMeta(state).label)).toEqual(["版本已知", "版本未知", "版本不兼容"]);
    expect(auth.map((state) => toolAuthMeta(state).label)).toEqual(["已登录", "未登录", "登录状态未知", "未支持认证探测"]);
    expect(integrations.map((state) => toolIntegrationMeta(state).label)).toEqual(["仅支持发现", "执行适配器可用", "平台不支持", "策略阻断"]);
    expect(toolRuntimeMeta(false, "execution.managed-policy-unverified").label).toBe("Runtime 被策略阻断");
    expect(toolRuntimeMeta(false, "tool.adapter-unavailable").label).toBe("无执行适配器");
  });

  it("distinguishes an unrun auth probe from probe and output failures", () => {
    expect(toolAuthMeta("unknown", "execution.auth-not-verified").label).toBe("尚未验证认证");
    expect(toolAuthMeta("unknown", "execution.auth-output-unrecognized").label).toBe("认证输出无法识别");
    expect(toolAuthMeta("unknown", "execution.auth-probe-timeout").label).toBe("认证探测超时");
    expect(toolAuthMeta("unknown", "execution.auth-probe-failed").label).toBe("认证探测失败");
    expect(toolAuthMeta("unknown", "execution.auth-cli-incompatible").label).toBe("认证探测不兼容");
  });
});
