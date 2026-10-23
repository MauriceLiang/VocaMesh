import { invoke } from "@tauri-apps/api/core";
import type { LocalTool } from "@/types";
import { ServiceError } from "@/types";
import type { ToolService } from "@/services/contracts";

interface IpcServiceError {
  code: string;
  message: string;
}

function mapError(error: unknown, fallbackCode: string, fallbackMessage: string): ServiceError {
  if (error && typeof error === "object" && "code" in error && "message" in error) {
    const value = error as IpcServiceError;
    if (typeof value.code === "string" && typeof value.message === "string") {
      return new ServiceError(value.message, value.code);
    }
  }
  return new ServiceError(fallbackMessage, fallbackCode);
}

async function call<T>(command: string, args?: Record<string, unknown>, fallbackCode = "tool.scan-failed", fallbackMessage = "本机工具服务暂时不可用"): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw mapError(error, fallbackCode, fallbackMessage);
  }
}

export function createTauriToolService(): ToolService {
  return {
    scanTools: () => call<LocalTool[]>("scan_tools", undefined, "tool.scan-failed", "本机工具扫描失败"),
    verifyTool: (id) => call<LocalTool>("verify_tool", { id }, "tool.version-unavailable", "工具验证失败"),
    setToolPath: (id, path) => call<LocalTool>("set_tool_path", { id, path }, "tool.invalid-path", "无法保存工具路径"),
    setDefaultTool: async (id) => {
      await call<void>("set_default_tool", { id }, "tool.adapter-unavailable", "该工具当前没有执行适配器");
    },
    getDefaultToolId: () => call<string | null>("get_default_tool_id", undefined, "settings.read-failed", "默认工具设置无法读取"),
  };
}
