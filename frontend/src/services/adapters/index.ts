import type { MockServices } from "@/services/mock/services";
import { createMockServices } from "@/services/mock/services";

/**
 * Service 适配层。
 *
 * 用途：页面/Store 只依赖这里的接口集合，不关心底层是 Mock 还是 Tauri IPC。
 * 当前：mode = "mock"，全部能力由内存 Mock 提供。
 * 未来：新增 `createTauriServices()`，把每个 Service 方法映射到 Rust Core 命令或事件。
 *
 * 注意：
 * - 不要在页面里判断 `import.meta.env` 后直接调用 Tauri API。
 * - 不要在这里伪造 Rust 命令名；对接方需要在 Rust 侧先实现命令再补映射。
 */

export type ServiceMode = "mock" | "tauri";

export interface Services extends MockServices {
  mode: ServiceMode;
  /** 数据来源说明，用于界面上的 “Mock 演示” 标识。 */
  sourceLabel: string;
}

let instance: Services | null = null;

export function createServices(mode: ServiceMode = "mock"): Services {
  if (mode === "tauri") {
    // 保留入口：真实实现接入时替换为 createTauriServices()
    throw new Error("Tauri Service 适配层尚未实现：请先在 Rust Core 提供命令与事件，再补充映射。");
  }
  return { ...createMockServices(), mode: "mock", sourceLabel: "Mock 演示数据" };
}

/** 单例访问：Store/页面统一使用该函数，便于未来替换实现。 */
export function useServices(): Services {
  if (!instance) instance = createServices("mock");
  return instance;
}

/** 仅测试使用：替换单例。 */
export function __setServicesForTesting(next: Services | null): void {
  instance = next;
}
