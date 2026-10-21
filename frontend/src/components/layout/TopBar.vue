<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Bell, CircleHelp, Moon, Search, Sun } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useNotificationsStore } from "@/stores/notifications";
import VocaIconButton from "@/components/ui/VocaIconButton.vue";

const props = defineProps<{ pageTitle: string; onOpenPalette: () => void }>();
const app = useAppStore();
const notifications = useNotificationsStore();
const route = useRoute();
const router = useRouter();

const themeIcon = computed(() => (app.resolvedTheme === "dark" ? Sun : Moon));
const themeLabel = computed(() => (app.resolvedTheme === "dark" ? "切换到亮色" : "切换到暗色"));

async function toggleTheme(): Promise<void> {
  await app.patchSettings({ theme: app.resolvedTheme === "dark" ? "light" : "dark" });
}
</script>

<template>
  <header class="v-toolbar">
    <span class="text-[var(--fs-body)] font-semibold">{{ props.pageTitle }}</span>
    <div class="v-spacer" />
    <button class="v-searchbox" aria-label="打开命令面板（⌘K）" @click="props.onOpenPalette">
      <Search :size="13" />
      <span>搜索任务、Run、Agent</span>
      <span class="v-spacer" />
      <span class="v-kbd">⌘K</span>
    </button>
    <VocaIconButton label="通知" @click="router.push({ name: 'notifications' })">
      <Bell :size="15" />
      <span
        v-if="notifications.unread"
        class="absolute right-1 top-1 h-[6px] w-[6px] rounded-full"
        :style="{ background: 'var(--text-primary)' }"
      />
    </VocaIconButton>
    <VocaIconButton :label="themeLabel" @click="toggleTheme">
      <component :is="themeIcon" :size="15" />
    </VocaIconButton>
    <VocaIconButton label="快捷键与帮助" @click="app.toast('快捷键：⌘K 命令面板 · Esc 关闭弹层 · Enter 发送消息', 'neutral')">
      <CircleHelp :size="15" />
    </VocaIconButton>
    <span class="v-avatar" :title="`本地账户（演示） · ${String(route.name ?? '')}`">M</span>
  </header>
</template>
