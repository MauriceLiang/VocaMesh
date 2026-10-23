import { afterEach, describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import AppTitleBar from "@/components/layout/AppTitleBar.vue";
import ApprovalsView from "@/features/approvals/ApprovalsView.vue";
import NotificationsView from "@/features/notifications/NotificationsView.vue";
import TasksView from "@/features/tasks/TasksView.vue";
import { __setServicesForTesting, createServices } from "@/services/adapters";
import { useApprovalsStore } from "@/stores/approvals";

function router() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/", name: "home", component: { template: "<div />" } }, { path: "/tasks", name: "tasks", component: { template: "<div />" } }],
  });
}

describe("native shell and unconnected views", () => {
  afterEach(() => __setServicesForTesting(null));

  it("hides mock titlebar controls and label in Tauri mode while keeping browser preview markers", async () => {
    setActivePinia(createPinia());
    __setServicesForTesting(createServices("tauri"));
    const native = mount(AppTitleBar, { props: { label: "测试页" } });
    expect(native.find(".v-lights").exists()).toBe(false);
    expect(native.find(".v-win-controls").exists()).toBe(false);
    expect(native.text()).not.toContain("mock 演示");
    native.unmount();

    setActivePinia(createPinia());
    __setServicesForTesting(createServices("mock"));
    const preview = mount(AppTitleBar, { props: { label: "测试页" } });
    expect(preview.find(".v-lights").exists()).toBe(true);
    expect(preview.text()).toContain("mock 演示");
  });

  it("renders safe empty approval and notification states in Tauri mode", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    __setServicesForTesting(createServices("tauri"));
    const approvalsStore = useApprovalsStore(pinia);
    approvalsStore.approvals = [{
      id: "stale_approval", taskId: "task_1", state: "pending", age: "刚刚", title: "git push --force",
      agent: "Demo Agent", target: "git push --force", diff: "fixture", scope: "one-time", risk: ["写入仓库"], history: [],
    }];
    approvalsStore.selectedId = "stale_approval";
    const nativeRouter = router();
    const approvals = mount(ApprovalsView, { global: { plugins: [nativeRouter, pinia] } });
    await flushPromises();
    expect(approvals.text()).toContain("本机审批尚未接入");
    expect(approvals.text()).not.toContain("仅此一次允许");
    expect(approvals.text()).not.toContain("git push --force");
    expect(approvals.text()).not.toContain("Demo Agent");
    approvals.unmount();

    setActivePinia(createPinia());
    const notifications = mount(NotificationsView, { global: { plugins: [router(), createPinia()] } });
    await flushPromises();
    expect(notifications.text()).toContain("暂无本机通知");
    expect(notifications.text()).not.toContain("CLI 认证成功");
  });

  it("uses no Mock task filters or fake subtasks in the native task list", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const services = createServices("tauri");
    services.events = {
      subscribeTaskLogs: () => () => undefined,
      subscribeTaskState: () => () => undefined,
      subscribeApprovals: () => () => undefined,
      subscribeToolScan: () => () => undefined,
    };
    services.tasks.listTasks = async () => [];
    services.agents.listAgents = async () => [];
    services.workspaces.listWorkspaces = async () => [];
    __setServicesForTesting(services);
    const nativeRouter = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: "/tasks", name: "tasks", component: TasksView }],
    });
    await nativeRouter.push("/tasks");
    await nativeRouter.isReady();

    const tasks = mount(TasksView, { global: { plugins: [nativeRouter, pinia] } });
    await flushPromises();

    expect(tasks.text()).toContain("暂无本机任务");
    expect(tasks.text()).not.toContain("research-2025");
    expect(tasks.text()).not.toContain("调研团队");
    expect(tasks.text()).not.toContain("仍使用演示数据");
  });
});
