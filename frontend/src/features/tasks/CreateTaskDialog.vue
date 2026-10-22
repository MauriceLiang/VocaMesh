<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { AlertTriangle, Check, Users, Workflow, X, Zap } from "lucide-vue-next";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaTextarea from "@/components/ui/VocaTextarea.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import type { CreateTaskInput } from "@/types";
import { useAgentsStore } from "@/stores/agents";
import { useTeamsStore } from "@/stores/teams";
import { useWorkspacesStore } from "@/stores/workspaces";

type Mode = CreateTaskInput["mode"];

const props = defineProps<{
  modelValue: boolean;
  defaultWorkspace?: string;
  initialDescription?: string;
  defaultMode?: Mode;
}>();
const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "submit", payload: CreateTaskInput): void;
}>();

const defaultDescription = "把 2025 Q3 三家竞品的公开资料整理成一份对比报告，标注数据来源；缺数据处留空并说明，不要推测。输出为 Markdown，并附一张对比表。";
const agents = useAgentsStore();
const teams = useTeamsStore();
const workspaces = useWorkspacesStore();
const description = ref(defaultDescription);
const workspace = ref(props.defaultWorkspace ?? "research-2025");
const mode = ref<Mode>("team");
const selectedTeamId = ref("");
const selectedAgentId = ref("");
const loadingOptions = ref(false);
const optionsError = ref<string | null>(null);

const availableAgents = computed(() => agents.agents.filter((agent) => agent.state === "ready"));
const availableTeams = computed(() => teams.teams.filter((team) => agents.agents.some((agent) => agent.id === team.supervisorId && agent.state === "ready")));
const workspaceItems = computed(() => workspaces.workspaces.map((item) => ({ value: item.name, label: `${item.name} · ${item.path}` })));
const agentItems = computed(() => availableAgents.value.map((agent) => ({ value: agent.id, label: `${agent.name} · ${agent.tool}` })));
const teamItems = computed(() => availableTeams.value.map((team) => ({ value: team.id, label: `${team.name} · ${team.members.length} 人` })));
const canSubmit = computed(() => {
  if (loadingOptions.value || optionsError.value || !description.value.trim() || !workspaceItems.value.some((item) => item.value === workspace.value)) return false;
  if (mode.value === "team") return teamItems.value.some((item) => item.value === selectedTeamId.value);
  if (mode.value === "single-agent") return agentItems.value.some((item) => item.value === selectedAgentId.value);
  return true;
});

async function prepare(): Promise<void> {
  description.value = props.initialDescription?.trim() ? props.initialDescription : defaultDescription;
  mode.value = props.defaultMode ?? "team";
  workspace.value = props.defaultWorkspace ?? "research-2025";
  selectedTeamId.value = "";
  selectedAgentId.value = "";
  optionsError.value = null;
  loadingOptions.value = true;
  try {
    await Promise.all([
      agents.agents.length ? Promise.resolve() : agents.load(),
      teams.teams.length ? Promise.resolve() : teams.load(),
      workspaces.workspaces.length ? Promise.resolve() : workspaces.load(),
    ]);
    if (!workspaceItems.value.some((item) => item.value === workspace.value)) workspace.value = workspaceItems.value[0]?.value ?? "";
    if (mode.value === "team") selectedTeamId.value = teamItems.value[0]?.value ?? "";
    if (mode.value === "single-agent") selectedAgentId.value = agentItems.value[0]?.value ?? "";
  } catch {
    optionsError.value = "无法加载团队、Agent 或工作空间，请关闭弹窗后重试。";
  } finally {
    loadingOptions.value = false;
  }
}

watch(() => props.modelValue, (open) => {
  if (open) void prepare();
});

const modes = computed(() => [
  { value: "single-agent" as const, label: "快速执行", description: "选择一个可用 Agent，不启动主管", icon: Zap },
  { value: "team" as const, label: "选择团队", description: "选择主管可用的团队", icon: Users },
  { value: "custom" as const, label: "自定义执行", description: "计划编辑尚未实现，仅创建待配置占位", icon: Workflow },
]);

function selectMode(next: Mode): void {
  mode.value = next;
  selectedTeamId.value = "";
  selectedAgentId.value = "";
  if (next === "team") selectedTeamId.value = teamItems.value[0]?.value ?? "";
  if (next === "single-agent") selectedAgentId.value = agentItems.value[0]?.value ?? "";
}

function selectWorkspace(next: string): void {
  workspace.value = next;
  selectedTeamId.value = "";
  selectedAgentId.value = "";
}

function submit(): void {
  if (!canSubmit.value) return;
  const base = { description: description.value.trim(), workspace: workspace.value };
  if (mode.value === "team") emit("submit", { ...base, mode: "team", teamId: selectedTeamId.value });
  else if (mode.value === "single-agent") emit("submit", { ...base, mode: "single-agent", agentId: selectedAgentId.value });
  else emit("submit", { ...base, mode: "custom" });
}
</script>

<template>
  <VocaDialog
    :model-value="modelValue"
    title="创建任务"
    :subtitle="mode === 'custom' ? '计划编辑尚未实现；此模式只会创建待配置占位任务，不会开始执行。' : '提交后先进入排队，运行环境检查通过才会开始执行。'"
    :width="760"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <div class="v-field">
      <span class="lbl">任务描述</span>
      <VocaTextarea v-model="description" :rows="3" aria-label="任务描述" />
    </div>

    <div class="v-field">
      <span class="lbl">工作空间</span>
      <VocaSelect :model-value="workspace" :items="workspaceItems" aria-label="工作空间" :disabled="loadingOptions || !workspaceItems.length" @update:model-value="selectWorkspace" />
    </div>

    <div class="v-banner is-blocked" :style="{ alignItems: 'flex-start' }">
      <AlertTriangle :size="13" />
      <span class="v-tiny" :style="{ lineHeight: 1.5 }">
        Mock 演示提示：Run #1244 使用 vocab-ops。该提示不扫描真实目录，也不代表本任务已完成冲突检查。
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
          @click="selectMode(item.value)"
        >
          <span class="flex items-center gap-[7px]">
            <component :is="item.icon" :size="14" />
            <b class="text-[12.5px] font-semibold">{{ item.label }}</b>
          </span>
          <span class="v-tiny v-muted" :style="{ lineHeight: 1.45 }">{{ item.description }}</span>
        </button>
      </div>
    </div>

    <div v-if="optionsError" class="v-banner is-failure"><AlertTriangle :size="13" /><span class="v-tiny">{{ optionsError }}</span></div>
    <div v-else-if="mode === 'team'" class="v-field">
      <span class="lbl">执行团队</span>
      <VocaSelect v-model="selectedTeamId" :items="teamItems" aria-label="执行团队" :disabled="loadingOptions || !teamItems.length" />
      <span v-if="!loadingOptions && !teamItems.length" class="v-tiny v-muted">暂无主管可用的团队，请先检查团队及其主管 Agent。</span>
      <span v-else-if="!loadingOptions && !selectedTeamId" class="v-tiny v-muted">请选择执行团队。</span>
    </div>
    <div v-else-if="mode === 'single-agent'" class="v-field">
      <span class="lbl">目标 Agent</span>
      <VocaSelect v-model="selectedAgentId" :items="agentItems" aria-label="目标 Agent" :disabled="loadingOptions || !agentItems.length" />
      <span v-if="!loadingOptions && !agentItems.length" class="v-tiny v-muted">暂无可用 Agent，请先检查本地工具和 Agent 状态。</span>
      <span v-else-if="!loadingOptions && !selectedAgentId" class="v-tiny v-muted">请选择目标 Agent。</span>
    </div>
    <div v-else class="v-banner is-neutral"><Workflow :size="13" /><span class="v-tiny">自定义计划编辑尚未实现。此模式只创建待配置占位任务，不会生成子任务或依赖，也不会开始 Mock 状态迁移。</span></div>

    <div class="v-card v-panel" :style="{ background: 'var(--bg-inset)', gap: '5px' }">
      <span class="lbl">本次任务的授权边界</span>
      <div class="flex items-center gap-[7px]"><Check :size="11" class="v-secondary" /><span class="v-tiny v-secondary">可读写 {{ workspace }}/ 及子目录；可创建新文件</span></div>
      <div class="flex items-center gap-[7px]"><Check :size="11" class="v-secondary" /><span class="v-tiny v-secondary">可读取公开网页（仅研究 Agent，限 20 次请求）</span></div>
      <div class="flex items-center gap-[7px]"><X :size="11" :style="{ color: 'var(--status-failure)' }" /><span class="v-tiny v-secondary">不可执行删除或 git push；不可访问工作空间外的目录</span></div>
    </div>

    <template #footer>
      <VocaButton variant="ghost" @click="emit('update:modelValue', false)">取消</VocaButton>
      <VocaButton variant="primary" :disabled="!canSubmit" :loading="loadingOptions" @click="submit">创建任务</VocaButton>
    </template>
  </VocaDialog>
</template>
