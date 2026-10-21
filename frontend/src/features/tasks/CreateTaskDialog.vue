<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { AlertTriangle, Check, Users, Workflow, X, Zap } from "lucide-vue-next";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaTextarea from "@/components/ui/VocaTextarea.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";

type Mode = "single-agent" | "team" | "custom";

const props = defineProps<{ modelValue: boolean; defaultWorkspace?: string }>();
const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "submit", payload: { description: string; workspace: string; mode: Mode }): void;
}>();

const description = ref(
  "把 2025 Q3 三家竞品的公开资料整理成一份对比报告，标注数据来源；缺数据处留空并说明，不要推测。输出为 Markdown，并附一张对比表。",
);
const workspace = ref(props.defaultWorkspace ?? "research-2025");
const mode = ref<Mode>("team");

watch(
  () => props.modelValue,
  (open) => {
    if (open) workspace.value = props.defaultWorkspace ?? "research-2025";
  },
);

const modes = computed(() => [
  { value: "single-agent" as const, label: "快速执行", description: "一个 Agent 直接完成，不启动主管", icon: Zap },
  { value: "team" as const, label: "选择团队", description: "主管协调多个 Agent 分工与交接", icon: Users },
  { value: "custom" as const, label: "自定义执行", description: "手动定义子任务、依赖与授权", icon: Workflow },
]);

const workspaceItems = [
  { value: "research-2025", label: "research-2025 · ~/work/research-2025" },
  { value: "vocab-ops", label: "vocab-ops · ~/work/vocab-ops" },
  { value: "release-2.4", label: "release-2.4 · ~/work/release-2.4" },
  { value: "legal-docs", label: "legal-docs · ~/work/legal-docs" },
];

function submit(): void {
  emit("submit", { description: description.value, workspace: workspace.value, mode: mode.value });
}
</script>

<template>
  <VocaDialog
    :model-value="modelValue"
    title="创建任务"
    subtitle="提交后先进入排队，运行环境检查通过才会开始执行。"
    :width="760"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <div class="v-field">
      <span class="lbl">任务描述</span>
      <VocaTextarea v-model="description" :rows="3" aria-label="任务描述" />
    </div>

    <div class="v-field">
      <span class="lbl">工作空间</span>
      <VocaSelect v-model="workspace" :items="workspaceItems" aria-label="工作空间" />
    </div>

    <div class="v-banner is-blocked" :style="{ alignItems: 'flex-start' }">
      <AlertTriangle :size="13" />
      <span class="v-tiny" :style="{ lineHeight: 1.5 }">
        检测到另一任务（Run #1244）正在写入 vocab-ops。本任务使用 {{ workspace }}，当前无冲突；切换目录会提示文件冲突风险。
      </span>
    </div>

    <div class="v-field">
      <span class="lbl">执行方式</span>
      <div class="grid grid-cols-3 gap-[10px]">
        <button
          v-for="item in modes"
          :key="item.value"
          type="button"
          class="flex flex-col gap-[5px] rounded-[8px] text-left"
          :style="{
            border: mode === item.value ? '2px solid var(--text-primary)' : '1px solid var(--border-default)',
            padding: mode === item.value ? '11px' : '12px',
            background: 'var(--bg-surface)',
          }"
          @click="mode = item.value"
        >
          <span class="flex items-center gap-[7px]">
            <component :is="item.icon" :size="14" />
            <b class="text-[12.5px] font-semibold">{{ item.label }}</b>
          </span>
          <span class="v-tiny v-muted" :style="{ lineHeight: 1.45 }">{{ item.description }}</span>
        </button>
      </div>
    </div>

    <div class="v-card v-panel" :style="{ background: 'var(--bg-inset)', gap: '5px' }">
      <span class="lbl">本次任务的授权边界</span>
      <div class="flex items-center gap-[7px]"><Check :size="11" class="v-secondary" /><span class="v-tiny v-secondary">可读写 {{ workspace }}/ 及子目录；可创建新文件</span></div>
      <div class="flex items-center gap-[7px]"><Check :size="11" class="v-secondary" /><span class="v-tiny v-secondary">可读取公开网页（仅研究 Agent，限 20 次请求）</span></div>
      <div class="flex items-center gap-[7px]"><X :size="11" :style="{ color: 'var(--status-failure)' }" /><span class="v-tiny v-secondary">不可执行删除或 git push；不可访问工作空间外的目录</span></div>
    </div>

    <template #footer>
      <VocaButton variant="ghost" @click="emit('update:modelValue', false)">取消</VocaButton>
      <VocaButton variant="primary" @click="submit">创建任务</VocaButton>
    </template>
  </VocaDialog>
</template>
