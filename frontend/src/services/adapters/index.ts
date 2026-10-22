import { isTauri } from "@tauri-apps/api/core";
import type { Services } from "@/services/contracts";
import { createMockServices } from "@/services/mock/services";
import { createTauriToolScanEvents } from "@/services/tauri/events";
import { createTauriToolService } from "@/services/tauri/tools";
import { createTauriExecutionService, createTauriRunService, createTauriRuntimeEvents, createTauriTaskService } from "@/services/tauri/execution";

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
    const execution = createTauriExecutionService();
    return {
      ...mock,
      tools: createTauriToolService(),
      tasks: createTauriTaskService(execution),
      runs: createTauriRunService(execution),
      events: createTauriRuntimeEvents(execution, mock.events, nativeEvents.subscribeToolScan),
      execution,
      mode: "tauri",
      sourceLabel: "本机 Claude 只读任务、Run 与日志已接入；Agent、团队、审批等仍为演示数据",
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
