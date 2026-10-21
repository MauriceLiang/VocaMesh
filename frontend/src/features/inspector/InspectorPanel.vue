<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Copy, FileText, Search } from "lucide-vue-next";
import type { LogLine, Task } from "@/types";
import VocaInspector from "@/components/layout/VocaInspector.vue";
import VocaTabs from "@/components/ui/VocaTabs.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import { useServices } from "@/services/adapters";
import { useAppStore } from "@/stores/app";
import { subtaskStateMeta } from "@/lib/status";

const props = defineProps<{
  task: Task;
  subtaskId: string | null;
  tab: "activity" | "logs" | "results" | "info";
}>();
const emit = defineEmits<{
  (event: "update:tab", value: "activity" | "logs" | "results" | "info"): void;
  (event: "close"): void;
  (event: "locate"): void;
}>();

const services = useServices();
const app = useAppStore();

const logs = ref<LogLine[]>([]);
const query = ref("");
const level = ref<"全部" | "警告" | "错误">("全部");
const autoScroll = ref(true);
const bodyRef = ref<HTMLElement | null>(null);
let unsubscribe: (() => void) | null = null;

const subtask = computed(() => props.task.subtasks.find((item) => item.id === props.subtaskId) ?? props.task.subtasks[0]);
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return logs.value.filter((line) => {
    if (level.value === "警告" && line.level !== "WARN") return false;
    if (level.value === "错误" && line.level !== "ERROR") return false;
    return !q || `${line.actor}${line.message}`.toLowerCase().includes(q);
  });
});

const results = ref<Array<{ path: string; state: string; note: string }>>([]);

onMounted(async () => {
  results.value = await services.runs.listArtifacts(props.task.runId);
  unsubscribe = services.events.subscribeTaskLogs(props.task.id, (line) => {
    logs.value = [...logs.value, line].slice(-200);
    if (autoScroll.value) {
      requestAnimationFrame(() => {
        if (bodyRef.value) bodyRef.value.scrollTop = bodyRef.value.scrollHeight;
      });
    }
  });
});

onUnmounted(() => unsubscribe?.());

function copyLogs(): void {
  const text = filtered.value.map((line) => `${line.time}  ${line.level}  ${line.actor}  ${line.message}`).join("\n");
  void navigator.clipboard?.writeText(text).then(
    () => app.toast(`已复制 ${filtered.value.length} 行日志`, "success"),
    () => app.toast("当前环境不允许写入剪贴板", "blocked"),
  );
}

const tabs = [
  { value: "activity" as const, label: "活动" },
  { value: "logs" as const, label: "日志" },
  { value: "results" as const, label: "成果" },
  { value: "info" as const, label: "信息" },
];
</script>

<template>
  <VocaInspector :title="subtask?.name ?? '检查器'" :subtitle="subtask?.assignee ?? ''" @close="emit('close')">
    <template #tabs>
      <VocaTabs :model-value="tab" :items="tabs" @update:model-value="emit('update:tab', $event)" />
    </template>

    <div v-if="tab === 'activity'" class="flex flex-col gap-[9px] overflow-auto px-[14px] py-3">
      <div v-for="item in task.subtasks" :key="item.id" class="flex items-start gap-2">
        <span class="v-mono v-tiny v-muted" :style="{ width: '44px' }">{{ item.id.toUpperCase() }}</span>
        <span class="flex flex-1 flex-col gap-[1px]">
          <span class="v-small">{{ item.name }}</span>
          <span class="v-tiny v-muted">{{ item.assignee }} · {{ item.result }}</span>
        </span>
        <VocaStatus :meta="subtaskStateMeta(item.state)" />
      </div>
    </div>

    <div v-else-if="tab === 'logs'" class="flex h-full flex-col">
      <div class="flex flex-col gap-2 px-[14px] py-[10px]">
        <div class="flex items-center gap-2">
          <label class="v-searchbox" :style="{ flex: '1 1 auto', minWidth: 0 }">
            <Search :size="12" />
            <input
              :value="query"
              class="w-full bg-transparent text-[11.5px] outline-none"
              placeholder="筛选日志关键字"
              aria-label="筛选日志关键字"
              @input="query = ($event.target as HTMLInputElement).value"
            />
          </label>
          <button class="flex items-center gap-[6px]" :aria-pressed="autoScroll" @click="autoScroll = !autoScroll">
            <span class="v-switch" :class="{ 'is-on': autoScroll }" />
            <span class="v-tiny v-secondary">自动滚动</span>
          </button>
        </div>
        <div class="flex items-center gap-[6px]">
          <button
            v-for="item in ['全部', '警告', '错误'] as const"
            :key="item"
            class="v-badge"
            :class="level === item ? 'is-neutral' : ''"
            :style="{ height: '22px', fontSize: '10.5px', padding: '0 8px', background: level === item ? 'var(--text-primary)' : 'var(--bg-inset)', color: level === item ? 'var(--text-inverse)' : 'var(--text-secondary)' }"
            @click="level = item"
          >
            {{ item }}
          </button>
          <span class="v-spacer" />
          <span class="v-tiny v-muted">{{ filtered.length }} 行 · 演示数据</span>
        </div>
      </div>
      <div class="v-divider" />
      <div ref="bodyRef" class="min-h-0 flex-1 overflow-auto py-[6px]" :style="{ fontFamily: 'var(--font-mono)', fontSize: '10.5px' }">
        <div
          v-for="(line, index) in filtered"
          :key="`${line.time}-${index}`"
          class="grid items-start gap-2 px-[14px] py-[3px]"
          :style="{ gridTemplateColumns: '52px 40px 68px 1fr', background: line.level === 'WARN' ? 'var(--status-blocked-bg)' : 'transparent' }"
        >
          <span class="v-muted">{{ line.time }}</span>
          <span :style="{ color: line.level === 'WARN' ? 'var(--status-blocked)' : 'var(--text-secondary)' }">{{ line.level }}</span>
          <span class="v-muted">{{ line.actor }}</span>
          <span :style="{ color: 'var(--text-primary)', wordBreak: 'break-word' }">{{ line.message }}</span>
        </div>
      </div>
      <div class="v-divider" />
      <div class="flex items-center gap-[10px] px-[14px] py-2">
        <span class="v-tiny v-muted">显示 {{ filtered.length }} / 231 行</span>
        <span class="v-spacer" />
        <VocaButton size="sm" variant="ghost" @click="copyLogs"><Copy :size="12" />复制</VocaButton>
        <VocaButton size="sm" variant="ghost" @click="app.toast('已加载更早的日志（演示）', 'neutral')">加载更早</VocaButton>
      </div>
    </div>

    <div v-else-if="tab === 'results'" class="flex flex-col gap-[9px] overflow-auto px-[14px] py-3">
      <div v-for="item in results" :key="item.path" class="v-card flex items-center gap-2" :style="{ padding: '8px 10px' }">
        <FileText :size="13" class="v-muted" />
        <span class="flex min-w-0 flex-1 flex-col gap-[1px]">
          <span class="v-mono" :style="{ fontSize: '11px' }">{{ item.path }}</span>
          <span class="v-tiny v-muted">{{ item.note }}</span>
        </span>
        <VocaButton size="sm" variant="ghost" @click="app.toast('文件预览：演示环境不读取真实文件', 'neutral')">预览</VocaButton>
      </div>
      <div class="v-tiny v-muted">路径为模拟路径，不会访问真实文件。</div>
    </div>

    <div v-else class="flex flex-col gap-2 overflow-auto px-[14px] py-3">
      <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">Agent 角色</span><span>{{ subtask?.assignee }}</span></div>
      <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">工作目录</span><span class="v-mono">~/work/{{ task.workspace }}</span></div>
      <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">权限边界</span><span>读: {{ task.workspace }}/ · 写: exports/ · 禁止: ~/.ssh</span></div>
      <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">关联 Run</span><span class="v-mono">{{ task.runId }}</span></div>
      <div class="v-divider" />
      <div class="flex items-center gap-2">
        <VocaButton size="sm" @click="emit('update:tab', 'logs')">查看日志</VocaButton>
        <VocaButton size="sm" variant="ghost" @click="emit('locate')">在时间线定位</VocaButton>
      </div>
    </div>
  </VocaInspector>
</template>
