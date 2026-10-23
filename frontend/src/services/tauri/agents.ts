import { ServiceError, type Agent, type AgentPermissions, type CreateAgentInput, type UpdateAgentInput } from "@/types";
import type { AgentService } from "@/services/contracts";
import { invokeNative } from "./ipc";

interface NativeAgent {
  id: string;
  name: string;
  role: string;
  providerId: string;
  description: string;
  instructions: string;
  permissions: AgentPermissions;
  revision: number;
}

const providerNames: Record<string, string> = { claude: "Claude Code", codex: "Codex CLI", opencode: "OpenCode" };
const defaultPermissions: AgentPermissions = { read: true, write: false, shell: false, network: false };

export function createTauriAgentService(): AgentService {
  const listNative = () => invokeNative<NativeAgent[]>("list_agents");
  function mapAgent(record: NativeAgent): Agent {
    return {
      id: record.id,
      name: record.name,
      role: record.role,
      tool: providerNames[record.providerId] ?? record.providerId,
      toolId: record.providerId,
      state: "not-ready",
      description: record.description,
      instructions: record.instructions,
      permissions: record.permissions,
      runtimeReason: "execution.managed-policy-unverified",
      revision: record.revision,
    };
  }

  return {
    async listAgents() {
      return (await listNative()).map(mapAgent);
    },
    async createAgent(input: CreateAgentInput) {
      const record = await invokeNative<NativeAgent>("create_agent", { input: {
        name: input.name,
        role: input.role,
        providerId: input.toolId,
        description: input.description ?? "",
        instructions: input.instructions ?? "",
        permissions: { ...defaultPermissions, ...input.permissions },
      } });
      return mapAgent(record);
    },
    async updateAgent(id: string, input: UpdateAgentInput) {
      const record = await invokeNative<NativeAgent>("update_agent", { id, input: {
        name: input.name,
        role: input.role,
        providerId: input.toolId,
        description: input.description,
        instructions: input.instructions,
        permissions: input.permissions ? { ...defaultPermissions, ...input.permissions } : undefined,
        expectedRevision: input.expectedRevision,
      } });
      return mapAgent(record);
    },
    async deleteAgent(id: string) { await invokeNative<void>("archive_agent", { id }); },
    async setPermissions(id: string, permissions: Partial<AgentPermissions>) {
      const current = (await listNative()).find((agent) => agent.id === id);
      if (!current) throw new ServiceError("找不到该 Agent 角色", "agent.not-found");
      const record = await invokeNative<NativeAgent>("set_agent_permissions", { id, permissions: { ...current.permissions, ...permissions }, expectedRevision: current.revision });
      return mapAgent(record);
    },
  };
}
