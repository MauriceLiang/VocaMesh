import { invoke } from "@tauri-apps/api/core";
import { ServiceError } from "@/types";

interface IpcError {
  code: string;
  message: string;
}

export async function invokeNative<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && "message" in error) {
      const ipc = error as IpcError;
      if (typeof ipc.code === "string" && typeof ipc.message === "string") {
        throw new ServiceError(ipc.message, ipc.code);
      }
    }
    throw new ServiceError("本机服务暂时不可用，请检查应用后重试", "native.ipc-failed");
  }
}
