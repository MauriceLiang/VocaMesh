import { isTauri } from "@tauri-apps/api/core";
import type { Services } from "@/services/contracts";
import { createMockServices } from "@/services/mock/services";
import { createTauriToolScanEvents } from "@/services/tauri/events";
import { createTauriToolService } from "@/services/tauri/tools";
import { createTauriExecutionService, createTauriRunService, createTauriRuntimeEvents, createTauriTaskService } from "@/services/tauri/execution";
import { createTauriAgentService } from "@/services/tauri/agents";
import { createTauriTeamService } from "@/services/tauri/teams";
import { createTauriWorkspaceService } from "@/services/tauri/workspaces";
import { createTauriSettingsService } from "@/services/tauri/settings";
import { createEmptyNativeNotificationService, createUnavailableApprovalService } from "@/services/tauri/unavailable";

export type ServiceMode = "mock" | "tauri";

export interface AppServices extends Services {
  mode: ServiceMode;
  /** Describes the active data source and capability boundary. */
  sourceLabel: string;
}

let instance: AppServices | null = null;

export function createServices(mode: ServiceMode = (isTauri() ? "tauri" : "mock")): AppServices {
  if (mode === "mock") return { ...createMockServices(), mode: "mock", sourceLabel: "Mock 演示数据" };

  const nativeEvents = createTauriToolScanEvents();
  const execution = createTauriExecutionService();
  const tools = createTauriToolService();
  return {
    mode: "tauri",
    sourceLabel: "本地配置与 Task/Run 数据已接入；真实执行仍受安全策略限制",
    tools,
    tasks: createTauriTaskService(execution),
    runs: createTauriRunService(execution),
    events: createTauriRuntimeEvents(execution, nativeEvents.subscribeToolScan),
    execution,
    agents: createTauriAgentService(),
    teams: createTauriTeamService(),
    workspaces: createTauriWorkspaceService(),
    approvals: createUnavailableApprovalService(),
    notifications: createEmptyNativeNotificationService(),
    settings: createTauriSettingsService(),
  };
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
