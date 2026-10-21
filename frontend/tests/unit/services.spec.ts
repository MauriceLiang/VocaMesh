import { beforeEach, describe, expect, it } from "vitest";
import { createMockServices, type MockServices } from "@/services/mock/services";

/**
 * Service 契约测试（Mock 实现）。
 * 目的：确认状态迁移与约束（排队→执行、暂停两段式、重试产生新 Run、审批一次性、团队环检测、工具不假定可用）。
 */
describe("Mock services", () => {
  let services: MockServices;
  beforeEach(() => {
    services = createMockServices();
  });

  it("创建任务先排队，再进入执行中", async () => {
    const task = await services.tasks.createTask({ description: "测试任务", workspace: "research-2025", mode: "team" });
    expect(task.state).toBe("queued");
    await new Promise((resolve) => setTimeout(resolve, 1600));
    const updated = await services.tasks.getTask(task.id);
    expect(updated.state).toBe("running");
  });

  it("暂停是两段式：等待暂停 → 已暂停", async () => {
    const [first] = await services.tasks.listTasks();
    const pending = await services.tasks.pauseTask(first.id);
    expect(pending.state).toBe("pause-pending");
    await new Promise((resolve) => setTimeout(resolve, 1800));
    const paused = await services.tasks.getTask(first.id);
    expect(paused.state).toBe("paused");
  });

  it("重试失败部分会创建新 Run 并保留旧 Run 编号", async () => {
    const tasks = await services.tasks.listTasks();
    const failed = tasks.find((task) => task.state === "failed");
    expect(failed).toBeDefined();
    const before = failed!.runId;
    const retried = await services.tasks.retryFailed(failed!.id);
    expect(retried.runId).not.toBe(before);
    expect(retried.stateNote).toContain(before);
  });

  it("审批只允许一次，并让任务恢复执行；二次决定被拒绝", async () => {
    const approvals = await services.approvals.listApprovals();
    const pending = approvals.find((item) => item.state === "pending")!;
    const decided = await services.approvals.decide(pending.id, "allow-once");
    expect(decided.state).toBe("allowed-once");
    const task = await services.tasks.getTask(pending.taskId);
    expect(["running", "needs-attention"]).toContain(task.state);
    await expect(services.approvals.decide(pending.id, "deny")).rejects.toThrow();
  });

  it("团队设置上级会阻止间接环", async () => {
    const team = await services.teams.getTeam("tm1");
    const supervisor = team.supervisorId;
    const member = team.members.find((item) => item.reportsTo === supervisor)!;
    await expect(services.teams.setReportsTo("tm1", supervisor, member.agentId)).rejects.toMatchObject({
      code: "team.cycle",
    });
  });

  it("工具无法确认时不会假定可用", async () => {
    const list = await services.tools.scanTools();
    const undetectable = list.find((tool) => tool.state === "undetectable");
    expect(undetectable).toBeDefined();
    const after = await services.tools.verifyTool(undetectable!.id);
    expect(after.state).toBe("undetectable");
  });

  it("数据跨 Service 一致：新建角色后团队与任务仍使用同一份数据", async () => {
    const agent = await services.agents.createAgent({ name: "核查员", role: "核对来源", toolId: "claude" });
    const agents = await services.agents.listAgents();
    expect(agents.some((item) => item.id === agent.id)).toBe(true);
    const team = await services.teams.addMember("tm1", { agentId: agent.id, reportsTo: "r1" });
    expect(team.members.some((member) => member.agentId === agent.id)).toBe(true);
  });
});
