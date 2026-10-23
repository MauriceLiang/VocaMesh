<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { Bell, Check, Pause, ShieldCheck, X } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import { useNotificationsStore } from "@/stores/notifications";
import { useServices } from "@/services/adapters";

const notifications = useNotificationsStore();
const services = useServices();
const isNative = services.mode === "tauri";
const router = useRouter();

onMounted(async () => {
  if (!notifications.items.length) await notifications.load();
});

function iconFor(kind: string) {
  return kind === "failure" ? X : kind === "success" ? Check : kind === "blocked" ? ShieldCheck : Pause;
}

function colorFor(kind: string) {
  return kind === "failure" ? "var(--status-failure)" : kind === "success" ? "var(--status-success)" : kind === "blocked" ? "var(--status-blocked)" : "var(--text-secondary)";
}

async function open(id: string): Promise<void> {
  const item = notifications.items.find((entry) => entry.id === id);
  await notifications.markRead(id);
  if (item?.target.screen) await router.push({ name: item.target.screen === "tasks" ? "tasks" : item.target.screen });
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="通知" :subtitle="isNative ? '本机通知事件源尚未接入；当前不会显示演示消息。' : '需要你关注的任务事件。通知只负责告诉你，审批操作仍在“待审批”中完成。'">
      <template #actions>
        <VocaButton v-if="!isNative" :disabled="!notifications.unread" @click="notifications.markAllRead()">全部标为已读</VocaButton>
      </template>
    </PageHeader>

    <div class="v-tabs" :style="{ padding: 0 }">
      <button
        v-for="item in [{ value: 'all', label: '全部' }, { value: 'unread', label: '未读' }, { value: 'approval', label: '审批' }] as const"
        :key="item.value"
        :class="{ 'is-on': notifications.filter === item.value }"
        @click="notifications.filter = item.value"
      >
        {{ item.label }}<template v-if="item.value === 'unread' && notifications.unread"> · {{ notifications.unread }}</template>
      </button>
    </div>

    <div class="v-card" :style="{ flex: '1 1 auto', overflow: 'auto' }">
      <div
        v-for="item in notifications.visible"
        :key="item.id"
        class="flex items-center gap-[11px] border-b px-[14px] py-[11px]"
        :style="{ borderColor: 'var(--border-default)', background: item.unread ? 'var(--bg-subtle)' : 'transparent' }"
      >
        <span class="grid h-[26px] w-[26px] place-items-center rounded-[6px] border" :style="{ background: 'var(--bg-surface)', borderColor: 'var(--border-default)', color: colorFor(item.kind) }">
          <component :is="iconFor(item.kind)" :size="13" />
        </span>
        <button class="flex min-w-0 flex-1 flex-col gap-[2px] text-left" @click="open(item.id)">
          <b class="v-small" :style="{ fontWeight: item.unread ? 600 : 500 }">{{ item.title }}</b>
          <span class="v-tiny v-muted">{{ item.body }}</span>
        </button>
        <span class="v-tiny v-muted">{{ item.time }}</span>
        <VocaButton size="sm" variant="ghost" @click="open(item.id)">查看</VocaButton>
        <button class="v-icon-btn" aria-label="移除通知" @click="notifications.dismiss(item.id)"><X :size="13" /></button>
        <span v-if="item.unread" class="h-[6px] w-[6px] rounded-full" :style="{ background: 'var(--text-primary)' }" />
      </div>
      <div v-if="!notifications.visible.length" class="v-empty">
        <Bell :size="18" class="v-muted" />
        <span class="text-[15px] font-semibold">{{ isNative ? "暂无本机通知" : "没有未读通知" }}</span>
        <span class="v-small">{{ isNative ? "真实通知源尚未接入，因此不会生成演示消息。" : "新的审批请求与任务事件会出现在这里。" }}</span>
      </div>
    </div>
  </div>
</template>
