<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { AlertTriangle, Check, Users, Workflow, X, Zap } from "lucide-vue-next";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaTextarea from "@/components/ui/VocaTextarea.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import VocaInput from "@/components/ui/VocaInput.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import RuntimeRecoveryPanel from "@/features/tasks/RuntimeRecoveryPanel.vue";
import type { CreateTaskInput, ExecutionReadiness, Task } from "@/types";
import { useAgentsStore } from "@/stores/agents";
import { useTeamsStore } from "@/stores/teams";
import { useWorkspacesStore } from "@/stores/workspaces";
import { useTasksStore } from "@/stores/tasks";
import { useToolsStore } from "@/stores/tools";
import { useServices } from "@/services/adapters";
import { toolAuthMeta, toolDiscoveryMeta, toolIntegrationMeta, toolRuntimeMeta, toolVersionMeta } from "@/lib/status";

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
  (event: "native-created", task: Task): void;
}>();

const defaultDescription = "把 2025 Q3 三家竞品的公开资料整理成一份对比报告，标注数据来源；缺数据处留空并说明，不要推测。输出为 Markdown，并附一张对比表。";
const agents = useAgentsStore();
const teams = useTeamsStore();
const workspaces = useWorkspacesStore();
const tasks = useTasksStore();
const tools = useToolsStore();
const services = useServices();
const isNative = services.mode === "tauri";
const description = ref(defaultDescription);
const workspace = ref(props.defaultWorkspace ?? "research-2025");
const mode = ref<Mode>("team");
const selectedTeamId = ref("");
const selectedAgentId = ref("");
const loadingOptions = ref(false);
const optionsError = ref<string | null>(null);
const nativeTitle = ref("只读目录分析");
const nativePrompt = ref("请总结所选工作目录的主要结构和用途。只根据实际读取到的内容回答。不要修改文件或执行命令。");
const nativeWorkspace = ref("");
const nativeProvider = ref<"claude" | "codex">("claude");
const nativeConsent = ref(false);
const nativeReadiness = ref<ExecutionReadiness | null>(null);
const nativeLoading = ref(false);
const nativeTool = computed(() => tools.tools.find((tool) => tool.id === nativeProvider.value) ?? null);
const nativeProviderName = computed(() => nativeProvider.value === "claude" ? "Claude Code" : "Codex CLI");
const nativeProviderItems = [
  { value: "claude", label: "Claude Code" },
  { value: "codex", label: "Codex CLI · 当前被安全策略阻断" },
];
const nativeSubtitle = computed(() => nativeProviderName.value + " 本机只读任务；仅在该 Provider 的 Runtime 策略、CLI 与认证检查全部通过时可提交。");
const preferredToolName = computed(() => {
  if (!tools.defaultToolId) return "未设置";
  return tools.tools.find((tool) => tool.id === tools.defaultToolId)?.name ?? tools.defaultToolId;
});

const availableAgents = computed(() => agents.agents.filter((agent) => agent.state === "ready"));
const availableTeams = computed(() => teams.teams.filter((team) => agents.agents.some((agent) => agent.id === team.supervisorId && agent.state === "ready")));
const workspaceItems = computed(() => workspaces.workspaces.map((item) => ({ value: item.name, label: `${item.name} · ${item.path}` })));
const agentItems = computed(() => availableAgents.value.map((agent) => ({ value: agent.id, label: `${agent.name} · ${agent.tool}` })));
const teamItems = computed(() => availableTeams.value.map((team) => ({ value: team.id, label: `${team.name} · ${team.members.length} 人` })));
const canSubmit = computed(() => {
  if (isNative) return !nativeLoading.value && !!nativeReadiness.value?.ready && nativeTitle.value.trim().length > 0 && nativePrompt.value.trim().length > 0 && !!nativeWorkspace.value && nativeConsent.value;
  if (loadingOptions.value || optionsError.value || !description.value.trim() || !workspaceItems.value.some((item) => item.value === workspace.value)) return false;
  if (mode.value === "team") return teamItems.value.some((item) => item.value === selectedTeamId.value);
  if (mode.value === "single-agent") return agentItems.value.some((item) => item.value === selectedAgentId.value);
  return true;
});

async function prepare(): Promise<void> {
  if (isNative) {
    optionsError.value = null;
    nativeTitle.value = "只读目录分析";
    nativePrompt.value = props.initialDescription?.trim() ? props.initialDescription : "请总结所选工作目录的主要结构和用途。只根据实际读取到的内容回答。不要修改文件或执行命令。";
    if (!workspaces.workspaces.length) await workspaces.load();
    nativeWorkspace.value = workspaces.workspaces.find((item) => item.name === props.defaultWorkspace)?.path ?? "";
    nativeProvider.value = "claude";
    nativeConsent.value = false;
    nativeReadiness.value = null;
    nativeLoading.value = true;
    let requestedProvider = nativeProvider.value;
    try {
      if (!tools.tools.length) await tools.load();
      requestedProvider = nativeProvider.value;
      const readiness = await services.execution.readiness(requestedProvider);
      if (nativeProvider.value === requestedProvider) nativeReadiness.value = readiness;
    }
    catch {
      if (nativeProvider.value === "claude") {
        nativeReadiness.value = { ready: false, version: null, authState: "unknown", message: "无法读取本机执行安全状态", reasonCode: "storage.read-failed" };
      }
    }
    finally {
      if (nativeProvider.value === requestedProvider) nativeLoading.value = false;
    }
    return;
  }
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
  if (isNative) {
    void submitNative();
    return;
  }
  if (!canSubmit.value) return;
  const base = { description: description.value.trim(), workspace: workspace.value };
  if (mode.value === "team") emit("submit", { ...base, mode: "team", teamId: selectedTeamId.value });
  else if (mode.value === "single-agent") emit("submit", { ...base, mode: "single-agent", agentId: selectedAgentId.value });
  else emit("submit", { ...base, mode: "custom" });
}

async function changeNativeProvider(value: string): Promise<void> {
  if (value !== "claude" && value !== "codex") return;
  const requestedProvider = value;
  nativeProvider.value = requestedProvider;
  nativeLoading.value = true;
  try {
    const readiness = await services.execution.readiness(requestedProvider);
    if (nativeProvider.value === requestedProvider) nativeReadiness.value = readiness;
  } catch {
    if (nativeProvider.value === requestedProvider) {
      nativeReadiness.value = {
        ready: false,
        version: null,
        authState: requestedProvider === "codex" ? "unsupported" : "unknown",
        message: "无法读取本机执行安全状态",
        reasonCode: "storage.read-failed",
      };
    }
  } finally {
    if (nativeProvider.value === requestedProvider) nativeLoading.value = false;
  }
}

async function chooseNativeWorkspace(): Promise<void> {
  try {
    const path = await services.execution.selectWorkspaceDirectory();
    if (path) nativeWorkspace.value = path;
  } catch {
    optionsError.value = "无法打开系统目录选择器，请重试。";
  }
}

async function submitNative(): Promise<void> {
  if (!canSubmit.value) return;
  nativeLoading.value = true;
  try {
    const task = await tasks.createReadOnly({
      requestId: crypto.randomUUID(),
      title: nativeTitle.value.trim(),
      prompt: nativePrompt.value.trim(),
      providerId: nativeProvider.value,
      workspacePath: nativeWorkspace.value,
      consent: nativeConsent.value,
    });
    if (task) {
      emit("native-created", task);
      emit("update:modelValue", false);
    }
  } finally {
    nativeLoading.value = false;
  }
}
</script>

<template>
  <VocaDialog
    :model-value="modelValue"
    title="创建任务"
    :subtitle="isNative ? nativeSubtitle : mode === 'custom' ? '计划编辑尚未实现；此模式只会创建待配置占位任务，不会开始执行。' : '提交后先进入排队，运行环境检查通过才会开始执行。'"
    :footer-hint="isNative ? '启动前必须通过运行策略检查并由你确认；Prompt 和读取到的内容可能发送至模型提供方。' : undefined"
    :width="760"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <template v-if="isNative">
      <div class="v-field">
        <span class="lbl">任务标题</span>
        <VocaInput v-model="nativeTitle" aria-label="任务标题" placeholder="例如：检查项目结构" :disabled="nativeLoading" />
      </div>
      <div class="v-field">
        <span class="lbl">Prompt</span>
        <VocaTextarea v-model="nativePrompt" :rows="4" aria-label="只读任务 Prompt" :disabled="nativeLoading" />
      </div>
      <div class="v-field">
        <span class="lbl">Provider</span>
        <VocaSelect v-model="nativeProvider" :items="nativeProviderItems" aria-label="执行 Provider" :disabled="nativeLoading" @update:model-value="changeNativeProvider" />
        <div class="v-card v-panel">
          <span class="v-small">{{ nativeProviderName }} · 本机 CLI</span>
          <span class="v-tiny v-muted">偏好工具：{{ preferredToolName }}；偏好不会绕过 Runtime 检查。</span>
          <div v-if="nativeTool" class="flex flex-wrap gap-[6px]">
            <VocaStatus :meta="toolDiscoveryMeta(nativeTool.discoveryStatus)" />
            <VocaStatus :meta="toolVersionMeta(nativeTool.versionStatus)" />
            <VocaStatus :meta="toolAuthMeta(nativeTool.authState)" />
            <VocaStatus :meta="toolIntegrationMeta(nativeTool.integrationStatus)" />
            <VocaStatus :meta="toolRuntimeMeta(nativeTool.runtimeReady, nativeTool.runtimeReasonCode)" />
          </div>
          <span v-if="nativeTool?.runtimeReasonCode" class="v-tiny v-muted">Runtime 原因：{{ nativeTool.runtimeReasonCode }}</span>
          <span class="v-tiny v-muted">{{ nativeProvider === 'codex' ? 'Codex JSONL 适配器已接入统一 Runtime；配置、凭据、文件、进程和网络边界未证明前，真实执行保持禁用。' : 'OpenCode 当前仅支持发现；Claude Code 仍受组织托管策略门禁约束。' }}</span>
        </div>
      </div>
      <div class="v-field">
        <span class="lbl">工作目录</span>
        <div class="flex items-center gap-2">
          <VocaInput :model-value="nativeWorkspace" aria-label="工作目录" placeholder="请选择本机目录" disabled />
          <VocaButton variant="ghost" :disabled="nativeLoading" @click="chooseNativeWorkspace">选择目录</VocaButton>
        </div>
      </div>
      <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">
        <AlertTriangle :size="13" />
        <span class="v-tiny" :style="{ lineHeight: 1.5 }">当前执行范围为只读快照，不向原工作目录写入；读取到的文件、Prompt 和输出可能发送给 {{ nativeProviderName }} 模型提供方并保存在本机任务历史中。目录过滤无法识别所有秘密，请勿在 Prompt 中填写密码或 Token。</span>
      </div>
      <label class="flex items-start gap-2 text-[11.5px]">
        <input v-model="nativeConsent" type="checkbox" :disabled="nativeLoading" />
        <span>我确认所选目录和只读范围，并同意将任务所需内容发送给当前 Provider 的模型提供方；我知道 Prompt、结果和目录路径会保存到本机应用数据。</span>
      </label>
      <div class="v-banner" :class="nativeReadiness?.ready ? 'is-neutral' : 'is-blocked'">
        <span class="v-tiny">{{ nativeLoading ? '正在检查当前 Provider 的版本、认证与独立执行策略…' : nativeReadiness?.message ?? '正在检查执行条件' }}</span>
        <span v-if="nativeReadiness?.reasonCode" class="v-mono v-tiny">{{ nativeReadiness.reasonCode }}</span>
      </div>
      <RuntimeRecoveryPanel v-if="nativeReadiness?.reasonCode === 'execution.process-state-unknown'" />
      <div v-if="optionsError" class="v-banner is-failure"><AlertTriangle :size="13" /><span class="v-tiny">{{ optionsError }}</span></div>
    </template>
    <template v-else>
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
    </template>

    <template #footer>
      <VocaButton variant="ghost" @click="emit('update:modelValue', false)">取消</VocaButton>
      <VocaButton variant="primary" :disabled="!canSubmit" :loading="isNative ? nativeLoading : loadingOptions" @click="submit">{{ isNative ? '开始只读任务' : '创建任务' }}</VocaButton>
    </template>
  </VocaDialog>
</template>
