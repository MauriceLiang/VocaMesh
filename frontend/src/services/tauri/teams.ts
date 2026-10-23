import { ServiceError, type Team, type TeamMember } from "@/types";
import type { TeamService } from "@/services/contracts";
import { invokeNative } from "./ipc";

interface NativeTeamMember {
  agentId: string;
  name: string;
  role: string;
  providerId: string;
  reportsToAgentId: string | null;
}

interface NativeTeam {
  id: string;
  name: string;
  supervisorId: string;
  workspaceId: string;
  workspace: string;
  members: NativeTeamMember[];
  revision: number;
}

interface NativeWorkspace { id: string; name: string }

const providerNames: Record<string, string> = { claude: "Claude Code", codex: "Codex CLI", opencode: "OpenCode" };

export function createTauriTeamService(): TeamService {
  async function resolveWorkspaceId(workspace: string | undefined, workspaceId: string | undefined): Promise<string | undefined> {
    if (workspaceId) return workspaceId;
    if (!workspace) return undefined;
    const workspaces = await invokeNative<NativeWorkspace[]>("list_workspaces");
    return workspaces.find((item) => item.id === workspace || item.name === workspace)?.id;
  }

  function mapMember(member: NativeTeamMember): TeamMember {
    return {
      agentId: member.agentId,
      name: member.name,
      role: member.role,
      reportsTo: member.reportsToAgentId ?? "user",
      providerId: member.providerId,
      tool: providerNames[member.providerId] ?? member.providerId,
      state: "not-ready",
    };
  }

  async function mapTeam(record: NativeTeam): Promise<Team> {
    return {
      id: record.id,
      name: record.name,
      supervisorId: record.supervisorId,
      workspaceId: record.workspaceId,
      workspace: record.workspace,
      revision: record.revision,
      members: record.members.map(mapMember),
    };
  }

  return {
    async listTeams() {
      const records = await invokeNative<NativeTeam[]>("list_teams");
      return records.map((record) => ({
        id: record.id, name: record.name, supervisorId: record.supervisorId,
        workspaceId: record.workspaceId, workspace: record.workspace, revision: record.revision,
        members: record.members.map(mapMember),
      }));
    },
    async getTeam(id) { return mapTeam(await invokeNative<NativeTeam>("get_team", { id })); },
    async createTeam(input) {
      const workspaceId = await resolveWorkspaceId(input.workspace, input.workspaceId);
      if (!workspaceId) throw new ServiceError("请选择已登记的工作空间", "team.workspace-unavailable");
      return mapTeam(await invokeNative<NativeTeam>("create_team", { input: { name: input.name, supervisorId: input.supervisorId, workspaceId } }));
    },
    async updateTeam(id, input) {
      const workspaceId = await resolveWorkspaceId(input.workspace, input.workspaceId);
      if ((input.workspace || input.workspaceId) && !workspaceId) throw new ServiceError("请选择已登记的工作空间", "team.workspace-unavailable");
      return mapTeam(await invokeNative<NativeTeam>("update_team", { id, input: {
        name: input.name,
        supervisorId: input.supervisorId,
        workspaceId,
        expectedRevision: input.expectedRevision,
      } }));
    },
    async addMember(teamId, input) {
      return mapTeam(await invokeNative<NativeTeam>("add_team_member", { id: teamId, input: { agentId: input.agentId, reportsToAgentId: input.reportsTo } }));
    },
    async removeMember(teamId, agentId) {
      return mapTeam(await invokeNative<NativeTeam>("remove_team_member", { id: teamId, agentId }));
    },
    async setReportsTo(teamId, agentId, reportsTo) {
      return mapTeam(await invokeNative<NativeTeam>("set_team_reports_to", { id: teamId, agentId, reportsToAgentId: reportsTo }));
    },
    async listMembers(teamId) { return (await this.getTeam(teamId)).members; },
  };
}
