<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ChevronDown, ChevronRight, Trash2 } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaInput from "@/components/ui/VocaInput.vue";
import VocaTextarea from "@/components/ui/VocaTextarea.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import VocaSwitch from "@/components/ui/VocaSwitch.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import { useAgentsStore } from "@/stores/agents";
import { useToolsStore } from "@/stores/tools";
import { agentStateMeta } from "@/lib/status";
import type { AgentPermissions } from "@/types";
import { useServices } from "@/services/adapters";
import { toolStateMeta } from "@/lib/status";

const agents = useAgentsStore();
const tools = useToolsStore();
const services = useServices();
const isNative = services.mode === "tauri";

const advancedOpen = ref(false);
const execOpen = ref(false);
const deleteOpen = ref(false);
const createOpen = ref(false);
const createName = ref("");
const createRole = ref("");
const createToolId = ref("claude");
const draftName = ref("");
const draftRole = ref("");
const draftDescription = ref("");
const draftInstructions = ref("");

const selected = computed(() => agents.selected);
const supportedProviders = [
  { id: "claude", name: "Claude Code" },
  { id: "codex", name: "Codex CLI" },
  { id: "opencode", name: "OpenCode" },
] as const;
const toolItems = computed(() => isNative
  ? supportedProviders.map((provider) => {
      const tool = tools.tools.find((item) => item.id === provider.id);
      return { value: provider.id, label: `${provider.name}（${tool ? toolStateMeta(tool.state).label : "未检测到本机 CLI"}）` };
    })
  : tools.tools.map((tool) => ({ value: tool.id, label: `${tool.name}（${tool.state === "ready" ? "可用" : "需登录/未就绪"}）` })));

const permissionRows: Array<{ key: keyof AgentPermissions; name: string; description: string }> = [
  { key: "read", name: "读取工作空间文件", description: "本地保存的范围配置草稿" },
  { key: "write", name: "写入与修改文件", description: "当前 Runtime 未开放写入" },
  { key: "shell", name: "执行 shell 命令", description: "当前 Runtime 未开放 Shell" },
  { key: "network", name: "访问网络", description: "当前 Runtime 未开放网络访问" },
];

onMounted(async () => {
  await Promise.all([agents.load(), tools.load()]);
  if (agents.selected) {
    draftName.value = agents.selected.name;
    draftRole.value = agents.selected.role;
    draftDescription.value = agents.selected.description;
    draftInstructions.value = agents.selected.instructions;
  }
});

function syncDraft(): void {
  if (agents.selected) {
    draftName.value = agents.selected.name;
    draftRole.value = agents.selected.role;
    draftDescription.value = agents.selected.description;
    draftInstructions.value = agents.selected.instructions;
  }
}

async function save(): Promise<void> {
  if (!agents.selectedId) return;
  await agents.update(agents.selectedId, { name: draftName.value, role: draftRole.value, description: draftDescription.value, instructions: draftInstructions.value });
}

async function changeTool(toolId: string): Promise<void> {
  if (!agents.selectedId) return;
  await agents.update(agents.selectedId, { toolId });
}

async function createAgent(): Promise<void> {
  const agent = await agents.create({
    name: createName.value.trim() || `新角色 ${agents.agents.length + 1}`,
    role: createRole.value.trim() || "成员",
    toolId: createToolId.value,
  });
  if (agent) {
    createOpen.value = false;
    createName.value = "";
    createRole.value = "";
    syncDraft();
  }
}

async function confirmDelete(): Promise<void> {
  if (!agents.selectedId) return;
  await agents.remove(agents.selectedId);
  deleteOpen.value = false;
  syncDraft();
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="Agent 角色" subtitle="决定一个执行环境扮演什么职责。同一个工具可以支持多个角色，无需重复安装。">
      <template #actions>
        <VocaButton variant="primary" @click="createOpen = true">新建 Agent</VocaButton>
      </template>
    </PageHeader>

    <div v-if="isNative" class="v-banner is-neutral">这里管理本机 Agent 角色配置。角色已保存不代表可执行；当前 managed policy 尚未核验，执行保持阻断。</div>
    <div v-if="agents.lastError" class="v-banner is-failure"><span class="flex-1">{{ agents.lastError }}</span><VocaButton size="sm" @click="agents.load()">重试</VocaButton></div>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-col-list v-col" :style="{ gap: '8px', overflow: 'auto' }">
        <button
          v-for="agent in agents.agents"
          :key="agent.id"
          class="v-card v-row"
          :style="{ padding: agent.id === agents.selectedId ? '11px' : '12px', gap: '10px', textAlign: 'left', border: agent.id === agents.selectedId ? '2px solid var(--text-primary)' : '1px solid var(--border-default)' }"
          @click="agents.select(agent.id); syncDraft()"
        >
          <span class="grid h-7 w-7 place-items-center rounded-[7px] text-[12px] font-semibold" :style="{ background: 'var(--bg-subtle)', color: 'var(--text-secondary)' }">
            {{ agent.name.slice(0, 1) }}
          </span>
          <span class="flex min-w-0 flex-1 flex-col gap-[1px]">
            <b class="text-[12.5px] font-medium">{{ agent.name }}</b>
            <span class="v-tiny v-muted">{{ agent.role }}</span>
          </span>
          <VocaStatus :meta="agentStateMeta(agent.state)" />
        </button>
      </div>

      <div v-if="selected" class="v-grow v-card v-detail-panel">
        <div class="flex items-center gap-2">
          <span class="text-[15px] font-semibold">编辑 Agent 角色</span>
          <VocaStatus :meta="agentStateMeta(selected.state)" />
          <span class="v-spacer" />
          <VocaButton size="sm" variant="ghost" @click="deleteOpen = true"><Trash2 :size="12" />删除角色</VocaButton>
          <VocaButton size="sm" variant="primary" @click="save">保存修改</VocaButton>
        </div>

        <div class="grid grid-cols-2 gap-[10px]">
          <div class="v-field">
            <span class="lbl">名称</span>
            <VocaInput v-model="draftName" aria-label="名称" />
          </div>
          <div class="v-field">
            <span class="lbl">职责</span>
            <VocaInput v-model="draftRole" aria-label="职责" />
          </div>
          <div class="v-field">
            <span class="lbl">执行工具</span>
            <VocaSelect :model-value="selected.toolId" :items="toolItems" aria-label="执行工具" @update:model-value="changeTool" />
          </div>
        </div>

        <div class="v-field">
          <span class="lbl">角色描述</span>
          <VocaTextarea v-model="draftDescription" :rows="2" aria-label="角色描述" />
        </div>

        <div class="v-field">
          <button class="flex items-center gap-[6px]" @click="advancedOpen = !advancedOpen">
            <component :is="advancedOpen ? ChevronDown : ChevronRight" :size="13" class="v-muted" />
            <span class="lbl">角色指令（高级）</span>
            <span class="v-tiny v-muted">{{ advancedOpen ? "已展开" : "默认收起" }}</span>
          </button>
            <VocaTextarea v-if="advancedOpen" v-model="draftInstructions" :rows="4" aria-label="角色指令" />
        </div>

        <div class="v-field">
          <span class="lbl">权限范围</span>
          <div class="v-card" :style="{ overflow: 'hidden' }">
            <div v-for="row in permissionRows" :key="row.key" class="flex items-center gap-[9px] border-t px-[10px] py-[7px] first:border-t-0" :style="{ borderColor: 'var(--border-default)' }">
              <span class="flex flex-1 flex-col gap-[1px]">
                <span class="v-small">{{ row.name }}</span>
                <span class="v-tiny v-muted">{{ row.description }}</span>
              </span>
              <VocaSwitch
                :model-value="selected.permissions[row.key]"
                :label="row.name"
                :disabled="isNative && row.key !== 'read'"
                @update:model-value="agents.togglePermission(selected.id, row.key, $event)"
              />
            </div>
          </div>
        </div>

        <button v-if="!isNative" class="flex items-center gap-2 rounded-[6px] border px-[10px] py-[9px] text-left" :style="{ borderColor: 'var(--border-default)' }" @click="execOpen = !execOpen">
          <component :is="execOpen ? ChevronDown : ChevronRight" :size="13" class="v-muted" />
          <span class="v-small v-secondary flex-1">执行参数：并发、超时、重试策略</span>
          <span class="v-tiny v-muted">{{ execOpen ? "已展开" : "默认收起" }}</span>
        </button>
        <div v-if="execOpen && !isNative" class="grid grid-cols-3 gap-[10px]">
          <div class="v-field"><span class="lbl">并发</span><div class="v-select" :style="{ cursor: 'default' }"><span class="v-mono">2</span><span class="v-tiny v-muted">任务</span></div></div>
          <div class="v-field"><span class="lbl">单步超时</span><div class="v-select" :style="{ cursor: 'default' }"><span class="v-mono">15</span><span class="v-tiny v-muted">分钟</span></div></div>
          <div class="v-field"><span class="lbl">自动重试</span><div class="v-select" :style="{ cursor: 'default' }"><span>关闭</span><span class="v-tiny v-muted">需人工确认</span></div></div>
        </div>

        <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">
          {{ isNative ? "所有 Agent 权限均为配置草稿，不授予读取、写入、Shell 或网络能力；真实执行目前不可用。" : "保存只代表配置已保存；正在运行的 Run 不会静默采用新配置，改动从下一个新任务/新 Run 开始生效。" }}
        </div>
      </div>
      <div v-else class="v-grow v-card v-empty">
        <Boxes :size="18" class="v-muted" />
        <span class="text-[15px] font-semibold">{{ agents.lastError ? "无法读取本机 Agent 配置" : "尚无 Agent 角色" }}</span>
        <span class="v-small">{{ agents.lastError ?? "创建的角色会保存在本机；保存配置不会启动 Agent 进程。" }}</span>
        <VocaButton v-if="agents.lastError" size="sm" @click="agents.load()">重试</VocaButton>
        <VocaButton v-else size="sm" variant="primary" @click="createOpen = true">添加第一个 Agent 角色</VocaButton>
      </div>
    </div>

    <VocaDialog v-model="createOpen" title="新建 Agent 角色" :subtitle="isNative ? '保存本机角色配置；不会启动 CLI 或模型任务。' : '最少三项即可保存；高级配置可在创建后编辑。'" :width="560">
      <div class="v-field"><span class="lbl">名称</span><VocaInput v-model="createName" aria-label="角色名称" placeholder="例如：事实核查员" /></div>
      <div class="v-field"><span class="lbl">职责描述</span><VocaInput v-model="createRole" aria-label="职责描述" placeholder="例如：核对来源与数字" /></div>
      <div class="v-field"><span class="lbl">执行工具</span><VocaSelect v-model="createToolId" :items="toolItems" aria-label="执行工具" /></div>
      <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">{{ isNative ? "角色已保存后仍不可执行；managed policy 未核验前运行时保持阻断。" : "保存只代表配置已保存；角色需要使用“已安装且可用”的工具才能真正执行任务。" }}</div>
      <template #footer>
        <VocaButton variant="ghost" @click="createOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="createAgent">保存角色</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="deleteOpen" title="归档 Agent 角色" subtitle="归档后该角色不能再被分配；历史 Run 记录保留。" :width="520">
      <div class="v-banner is-failure" :style="{ alignItems: 'flex-start' }">
        将归档角色 <b>{{ selected?.name }}</b>。如果团队仍引用此角色，后端会阻止归档。
      </div>
      <template #footer>
        <VocaButton variant="ghost" @click="deleteOpen = false">取消</VocaButton>
        <VocaButton variant="danger" @click="confirmDelete">归档角色</VocaButton>
      </template>
    </VocaDialog>
  </div>
</template>
