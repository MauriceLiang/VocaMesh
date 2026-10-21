<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { CornerDownLeft, Search } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ (event: "update:modelValue", value: boolean): void }>();

const router = useRouter();
const app = useAppStore();
const query = ref("");
const index = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);

interface Command {
  label: string;
  route?: string;
  run?: () => void | Promise<void>;
}

const commands = computed<Command[]>(() => [
  { label: "新建任务", route: "dashboard" },
  { label: "全部任务", route: "tasks" },
  { label: "待审批", route: "approvals" },
  { label: "历史记录", route: "history" },
  { label: "本地工具", route: "tools" },
  { label: "Agent 角色", route: "agents" },
  { label: "Agent 团队", route: "teams" },
  { label: "工作空间", route: "workspaces" },
  { label: "设置", route: "settings" },
  { label: "帮助与反馈", route: "help" },
  { label: "切换亮/暗主题", run: () => app.patchSettings({ theme: app.resolvedTheme === "dark" ? "light" : "dark" }) },
  { label: "切换界面密度", run: () => app.patchSettings({ density: app.settings.density === "compact" ? "comfortable" : "compact" }) },
]);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return q ? commands.value.filter((item) => item.label.toLowerCase().includes(q)) : commands.value;
});

watch(
  () => props.modelValue,
  async (open) => {
    if (!open) return;
    query.value = "";
    index.value = 0;
    await nextTick();
    inputRef.value?.focus();
  },
);

async function run(item: Command | undefined): Promise<void> {
  if (!item) return;
  emit("update:modelValue", false);
  if (item.run) await item.run();
  else if (item.route) await router.push({ name: item.route });
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    index.value = (index.value + 1) % Math.max(1, filtered.value.length);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    index.value = (index.value - 1 + filtered.value.length) % Math.max(1, filtered.value.length);
  } else if (event.key === "Enter") {
    event.preventDefault();
    void run(filtered.value[index.value]);
  } else if (event.key === "Escape") {
    emit("update:modelValue", false);
  }
}
</script>

<template>
  <div v-if="modelValue" class="fixed inset-0 z-[80] grid place-items-start justify-center bg-black/25 pt-[12vh]" @click.self="emit('update:modelValue', false)">
    <div class="w-[min(560px,92vw)] overflow-hidden rounded-[8px] border" :style="{ background: 'var(--bg-surface)', borderColor: 'var(--border-default)', boxShadow: 'var(--shadow-dialog)' }">
      <div class="flex items-center gap-2 border-b px-4" :style="{ borderColor: 'var(--border-default)' }">
        <Search :size="14" class="v-muted" />
        <input
          ref="inputRef"
          v-model="query"
          class="flex-1 bg-transparent py-3 text-[14px] outline-none"
          placeholder="搜索页面或命令…"
          aria-label="搜索页面或命令"
          @keydown="onKeydown"
          @input="index = 0"
        />
      </div>
      <div class="max-h-[320px] overflow-auto p-1">
        <button
          v-for="(item, i) in filtered"
          :key="item.label"
          class="v-menu-item"
          :style="i === index ? { background: 'var(--bg-subtle)' } : undefined"
          @mouseenter="index = i"
          @click="run(item)"
        >
          <CornerDownLeft :size="12" class="v-muted" />
          <span class="flex-1 text-left">{{ item.label }}</span>
          <span v-if="item.route" class="v-tiny v-muted">{{ item.route }}</span>
        </button>
        <div v-if="!filtered.length" class="p-3 v-small v-muted">没有匹配项</div>
      </div>
    </div>
  </div>
</template>
