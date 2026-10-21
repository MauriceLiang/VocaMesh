import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

/**
 * 路由表：与原型 16 个页面一一对应。
 * 使用 hash 模式，便于 Tauri 以文件方式加载打包产物。
 */
const routes: RouteRecordRaw[] = [
  { path: "/welcome", name: "welcome", component: () => import("@/features/onboarding/WelcomeView.vue"), meta: { title: "首次启动", plain: true } },
  { path: "/", name: "dashboard", component: () => import("@/features/dashboard/DashboardView.vue"), meta: { title: "工作台" } },
  { path: "/tasks", name: "tasks", component: () => import("@/features/tasks/TasksView.vue"), meta: { title: "全部任务" } },
  { path: "/tasks/:id", name: "task-detail", component: () => import("@/features/tasks/TaskDetailView.vue"), meta: { title: "全部任务 · 任务详情" } },
  { path: "/approvals", name: "approvals", component: () => import("@/features/approvals/ApprovalsView.vue"), meta: { title: "待审批" } },
  { path: "/history", name: "history", component: () => import("@/features/history/HistoryView.vue"), meta: { title: "历史记录" } },
  { path: "/agents", name: "agents", component: () => import("@/features/agents/AgentsView.vue"), meta: { title: "Agent 角色" } },
  { path: "/tools", name: "tools", component: () => import("@/features/tools/ToolsView.vue"), meta: { title: "本地工具" } },
  { path: "/teams", name: "teams", component: () => import("@/features/teams/TeamsView.vue"), meta: { title: "Agent 团队" } },
  { path: "/workspaces", name: "workspaces", component: () => import("@/features/workspaces/WorkspacesView.vue"), meta: { title: "工作空间" } },
  { path: "/settings", name: "settings", component: () => import("@/features/settings/SettingsView.vue"), meta: { title: "设置" } },
  { path: "/notifications", name: "notifications", component: () => import("@/features/notifications/NotificationsView.vue"), meta: { title: "通知" } },
  { path: "/help", name: "help", component: () => import("@/features/help/HelpView.vue"), meta: { title: "帮助与反馈" } },
  { path: "/states", name: "states", component: () => import("@/features/states/StatesView.vue"), meta: { title: "异常与空状态" } },
  { path: "/:pathMatch(.*)*", redirect: { name: "dashboard" } },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
  scrollBehavior: () => ({ top: 0 }),
});
