import type { Workspace } from "@/types";
import { ServiceError } from "@/types";
import type { WorkspaceService } from "@/services/contracts";
import { invokeNative } from "./ipc";

interface NativeWorkspace {
  id: string;
  name: string;
  path: string;
  pathAvailable: boolean;
  taskCount: number;
  scopes: { read: string[]; write: string[]; deny: string[] };
  revision: number;
}

function mapWorkspace(record: NativeWorkspace): Workspace {
  return {
    id: record.id,
    name: record.name,
    path: record.path,
    pathAvailable: record.pathAvailable,
    taskCount: record.taskCount,
    state: "readonly",
    read: record.scopes.read,
    write: record.scopes.write,
    deny: record.scopes.deny,
    revision: record.revision,
    executionEnabled: false,
  };
}

export function createTauriWorkspaceService(): WorkspaceService {
  return {
    async listWorkspaces() { return (await invokeNative<NativeWorkspace[]>("list_workspaces")).map(mapWorkspace); },
    async createWorkspace(input) {
      return mapWorkspace(await invokeNative<NativeWorkspace>("create_workspace", { input }));
    },
    async renameWorkspace(id, name, expectedRevision) {
      const revision = expectedRevision ?? (await invokeNative<NativeWorkspace>("get_workspace", { id })).revision;
      return mapWorkspace(await invokeNative<NativeWorkspace>("rename_workspace", {
        id, name, expectedRevision: revision,
      }));
    },
    async updateBoundaries(id, input) {
      const current = await invokeNative<NativeWorkspace>("get_workspace", { id });
      const record = await invokeNative<NativeWorkspace>("update_workspace_scopes", {
        id,
        input: {
          scopes: { read: input.read, write: input.write, deny: current.scopes.deny },
          expectedRevision: input.expectedRevision ?? current.revision,
        },
      });
      return mapWorkspace(record);
    },
    async archiveWorkspace(id) { await invokeNative<void>("archive_workspace", { id }); },
    async resolveConflict(_id) {
      throw new ServiceError("本机目录冲突排队尚未接入", "feature.not-implemented");
    },
    async resolveOpenPath(id) {
      const workspace = await invokeNative<NativeWorkspace>("get_workspace", { id });
      return workspace.path;
    },
  };
}
