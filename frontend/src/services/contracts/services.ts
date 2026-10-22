import type { AgentService } from "./agent-service";
import type { ApprovalService } from "./approval-service";
import type { EventService } from "./event-service";
import type { ExecutionService } from "./execution-service";
import type { NotificationService } from "./notification-service";
import type { RunService } from "./run-service";
import type { SettingsService } from "./settings-service";
import type { TaskService } from "./task-service";
import type { TeamService } from "./team-service";
import type { ToolService } from "./tool-service";
import type { WorkspaceService } from "./workspace-service";

/** Complete app service surface; implementations can mix native and mock services. */
export interface Services {
  agents: AgentService;
  tools: ToolService;
  teams: TeamService;
  tasks: TaskService;
  approvals: ApprovalService;
  runs: RunService;
  workspaces: WorkspaceService;
  settings: SettingsService;
  notifications: NotificationService;
  events: EventService;
  execution: ExecutionService;
}
