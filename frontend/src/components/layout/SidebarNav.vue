<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import {
  Bell,
  Boxes,
  CircleHelp,
  Folder,
  History,
  LayoutDashboard,
  ListChecks,
  Settings,
  ShieldCheck,
  SquareTerminal,
  Users,
} from "lucide-vue-next";
import { useApprovalsStore } from "@/stores/approvals";
import { useNotificationsStore } from "@/stores/notifications";

/**
 * 左侧导航：固定展开（没有折叠按钮），分组 + 计数。
 * 与原型 S02 侧栏 1:1 对应：工作 / Agent 与协作 / 应用 + 底部设置、帮助与反馈。
 */
const route = useRoute();
const approvals = useApprovalsStore();
const notifications = useNotificationsStore();

const groups = [
  {
    title: "工作",
    items: [
      { name: "dashboard", label: "工作台", icon: LayoutDashboard },
      { name: "tasks", label: "全部任务", icon: ListChecks },
      { name: "approvals", label: "待审批", icon: ShieldCheck },
      { name: "history", label: "历史记录", icon: History },
    ],
  },
  {
    title: "Agent 与协作",
    items: [
      { name: "agents", label: "Agent 角色", icon: Boxes },
      { name: "tools", label: "本地工具", icon: SquareTerminal },
      { name: "teams", label: "Agent 团队", icon: Users },
      { name: "workspaces", label: "工作空间", icon: Folder },
    ],
  },
  {
    title: "应用",
    items: [{ name: "notifications", label: "通知", icon: Bell }],
  },
];

const footItems = [
  { name: "settings", label: "设置", icon: Settings },
  { name: "help", label: "帮助与反馈", icon: CircleHelp },
];

const counts = computed<Record<string, number>>(() => ({
  approvals: approvals.pending.length,
  notifications: notifications.unread,
}));

const isActive = (name: string) => route.name === name || (name === "tasks" && route.name === "task-detail");
</script>

<template>
  <aside class="v-sidebar" aria-label="主导航">
    <div class="v-brand">
      <span class="mark" aria-hidden="true" />
      <span class="txt">
        <span class="p">VocaMesh</span>
        <span class="b">by Vocafect</span>
      </span>
    </div>

    <template v-for="group in groups" :key="group.title">
      <div class="v-nav-group">{{ group.title }}</div>
      <RouterLink
        v-for="item in group.items"
        :key="item.name"
        :to="{ name: item.name }"
        class="v-nav-item"
        :class="{ 'is-active': isActive(item.name) }"
        :aria-current="isActive(item.name) ? 'page' : undefined"
      >
        <component :is="item.icon" :size="15" />
        <span class="flex-1 truncate">{{ item.label }}</span>
        <span v-if="counts[item.name]" class="v-nav-count">{{ counts[item.name] }}</span>
      </RouterLink>
    </template>

    <div class="v-nav-spacer" />
    <div class="v-sidebar-foot">
      <RouterLink
        v-for="item in footItems"
        :key="item.name"
        :to="{ name: item.name }"
        class="v-nav-item"
        :class="{ 'is-active': isActive(item.name) }"
        :aria-current="isActive(item.name) ? 'page' : undefined"
      >
        <component :is="item.icon" :size="15" />
        <span class="flex-1 truncate">{{ item.label }}</span>
      </RouterLink>
    </div>
  </aside>
</template>
