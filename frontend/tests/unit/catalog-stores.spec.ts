import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ServiceError } from "@/types";
import { __setServicesForTesting, createServices } from "@/services/adapters";
import { useWorkspacesStore } from "@/stores/workspaces";

describe("native catalog stores", () => {
  afterEach(() => __setServicesForTesting(null));

  it("keeps the workspace list empty and exposes a retryable error when native loading fails", async () => {
    const services = createServices("tauri");
    services.workspaces = {
      listWorkspaces: vi.fn().mockRejectedValue(new ServiceError("本机配置不可用", "storage.read-failed")),
      createWorkspace: vi.fn(),
      renameWorkspace: vi.fn(),
      updateBoundaries: vi.fn(),
      archiveWorkspace: vi.fn(),
      resolveConflict: vi.fn(),
      resolveOpenPath: vi.fn(),
    };
    setActivePinia(createPinia());
    __setServicesForTesting(services);

    const workspaces = useWorkspacesStore();
    await workspaces.load();

    expect(workspaces.workspaces).toEqual([]);
    expect(workspaces.lastError).toBe("本机配置不可用");
  });

  it("does not remove a workspace from Pinia when native archive is rejected", async () => {
    const services = createServices("tauri");
    const workspace = {
      id: "workspace_1", name: "Local", path: "/tmp/project", taskCount: 0,
      state: "readonly" as const, read: ["."], write: [], deny: [], revision: 1, executionEnabled: false,
    };
    services.workspaces = {
      listWorkspaces: vi.fn().mockResolvedValue([workspace]),
      createWorkspace: vi.fn(),
      renameWorkspace: vi.fn(),
      updateBoundaries: vi.fn(),
      archiveWorkspace: vi.fn().mockRejectedValue(new ServiceError("工作空间仍被团队引用", "workspace.in-use")),
      resolveConflict: vi.fn(),
      resolveOpenPath: vi.fn(),
    };
    setActivePinia(createPinia());
    __setServicesForTesting(services);

    const workspaces = useWorkspacesStore();
    await workspaces.load();
    await expect(workspaces.archive(workspace.id)).resolves.toBe(false);

    expect(workspaces.workspaces).toEqual([workspace]);
    expect(workspaces.selectedId).toBe(workspace.id);
  });

  it("sends the loaded revision with native workspace edits", async () => {
    const services = createServices("tauri");
    const workspace = {
      id: "workspace_2", name: "Local", path: "/tmp/project", taskCount: 0,
      state: "readonly" as const, read: ["."], write: [], deny: [], revision: 9, executionEnabled: false,
    };
    const updateBoundaries = vi.fn().mockResolvedValue({ ...workspace, read: ["src"], revision: 10 });
    const renameWorkspace = vi.fn().mockResolvedValue({ ...workspace, name: "Renamed", revision: 11 });
    services.workspaces = {
      listWorkspaces: vi.fn().mockResolvedValue([workspace]),
      createWorkspace: vi.fn(),
      renameWorkspace,
      updateBoundaries,
      archiveWorkspace: vi.fn(),
      resolveConflict: vi.fn(),
      resolveOpenPath: vi.fn(),
    };
    setActivePinia(createPinia());
    __setServicesForTesting(services);

    const workspaces = useWorkspacesStore();
    await workspaces.load();
    await workspaces.updateBoundaries(["src"], []);
    await workspaces.rename("Renamed");

    expect(updateBoundaries).toHaveBeenCalledWith(workspace.id, {
      read: ["src"], write: [], expectedRevision: 9,
    });
    expect(renameWorkspace).toHaveBeenCalledWith(workspace.id, "Renamed", 10);
  });
});
