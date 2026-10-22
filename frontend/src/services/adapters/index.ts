import { isTauri } from "@tauri-apps/api/core";
import type { Services } from "@/services/contracts";
import { createMockServices } from "@/services/mock/services";
import { createTauriToolScanEvents } from "@/services/tauri/events";
import { createTauriToolService } from "@/services/tauri/tools";

export type ServiceMode = "mock" | "tauri";

export interface AppServices extends Services {
  mode: ServiceMode;
  /** Tool discovery is real in desktop mode; other unimplemented domains remain demo data. */
  sourceLabel: string;
}

let instance: AppServices | null = null;

export function createServices(mode: ServiceMode = (isTauri() ? "tauri" : "mock")): AppServices {
  const mock = createMockServices();
  if (mode === "tauri") {
    const nativeEvents = createTauriToolScanEvents();
    return {
      ...mock,
      tools: createTauriToolService(),
      events: { ...mock.events, subscribeToolScan: nativeEvents.subscribeToolScan },
      mode: "tauri",
      sourceLabel: "本机工具检测已接入；任务、Agent、团队、审批和运行历史仍为演示数据",
    };
  }
  return { ...mock, mode: "mock", sourceLabel: "Mock 演示数据" };
}

/** Stores and pages use this boundary and never call native IPC directly. */
export function useServices(): AppServices {
  if (!instance) instance = createServices();
  return instance;
}

/** Only used by tests to replace the singleton. */
export function __setServicesForTesting(next: AppServices | null): void {
  instance = next;
}
