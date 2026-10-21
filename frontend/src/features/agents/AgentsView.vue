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

const agents = useAgentsStore();
const tools = useToolsStore();

const advancedOpen = ref(false);
const execOpen = ref(false);
const deleteOpen = ref(false);
const createOpen = ref(false);
const createName = ref("");
const createRole = ref("");
const createToolId = ref("claude");
const draftName = ref("");
const draftDescription = ref("");

const selected = computed(() => agents.selected);
const toolItems = computed(() => tools.tools.map((tool) => ({ value: tool.id, label: `${tool.name}（${tool.state === "ready" ? "可用" : "需登录/未就绪"}）` })));

const permissionRows: Array<{ key: keyof AgentPermissions; name: string; description: string }> = [
  { key: "read", name: "读取工作空间文件", description: "只能读取 research-2025 及子目录" },
  { key: "write", name: "写入与修改文件", description: "写入前对高风险文件请求授权" },
  { key: "shell", name: "执行 shell 命令", description: "白名单命令可直接执行" },
  { key: "network", name: "访问网络", description: "仅研究类角色可开启，限 20 次/任务" },
];

onMounted(async () => {
  await Promise.all([agents.load(), tools.load()]);
  if (agents.selected) {
    draftName.value = agents.selected.name;
    draftDescription.value = agents.selected.description;
  }
});

function syncDraft(): void {
  if (agents.selected) {
    draftName.value = agents.selected.name;
    draftDescription.value = agents.selected.description;
  }
}

async function save(): Promise<void> {
  if (!agents.selectedId) return;
  await agents.update(agents.selectedId, { name: draftName.value, description: draftDescription.value });
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
          <VocaTextarea v-if="advancedOpen" :model-value="selected.instructions" :rows="4" aria-label="角色指令" />
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
                @update:model-value="agents.togglePermission(selected.id, row.key, $event)"
              />
            </div>
          </div>
        </div>

        <button class="flex items-center gap-2 rounded-[6px] border px-[10px] py-[9px] text-left" :style="{ borderColor: 'var(--border-default)' }" @click="execOpen = !execOpen">
          <component :is="execOpen ? ChevronDown : ChevronRight" :size="13" class="v-muted" />
          <span class="v-small v-secondary flex-1">执行参数：并发、超时、重试策略</span>
          <span class="v-tiny v-muted">{{ execOpen ? "已展开" : "默认收起" }}</span>
        </button>
        <div v-if="execOpen" class="grid grid-cols-3 gap-[10px]">
          <div class="v-field"><span class="lbl">并发</span><div class="v-select" :style="{ cursor: 'default' }"><span class="v-mono">2</span><span class="v-tiny v-muted">任务</span></div></div>
          <div class="v-field"><span class="lbl">单步超时</span><div class="v-select" :style="{ cursor: 'default' }"><span class="v-mono">15</span><span class="v-tiny v-muted">分钟</span></div></div>
          <div class="v-field"><span class="lbl">自动重试</span><div class="v-select" :style="{ cursor: 'default' }"><span>关闭</span><span class="v-tiny v-muted">需人工确认</span></div></div>
        </div>

        <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">
          保存只代表配置已保存；正在运行的 Run 不会静默采用新配置，改动从下一个新任务/新 Run 开始生效。
        </div>
      </div>
    </div>

    <VocaDialog v-model="createOpen" title="新建 Agent 角色" subtitle="最少三项即可保存；高级配置可在创建后编辑。" :width="560">
      <div class="v-field"><span class="lbl">名称</span><VocaInput v-model="createName" aria-label="角色名称" placeholder="例如：事实核查员" /></div>
      <div class="v-field"><span class="lbl">职责描述</span><VocaInput v-model="createRole" aria-label="职责描述" placeholder="例如：核对来源与数字" /></div>
      <div class="v-field"><span class="lbl">执行工具</span><VocaSelect v-model="createToolId" :items="toolItems" aria-label="执行工具" /></div>
      <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">保存只代表配置已保存；角色需要使用“已安装且可用”的工具才能真正执行任务。</div>
      <template #footer>
        <VocaButton variant="ghost" @click="createOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="createAgent">保存角色</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="deleteOpen" title="删除 Agent 角色" subtitle="删除后该角色不能再被分配；历史 Run 记录保留。" :width="520">
      <div class="v-banner is-failure" :style="{ alignItems: 'flex-start' }">
        将删除角色 <b>{{ selected?.name }}</b>。该角色不能再被分配；历史 Run、审批记录与日志保留。
      </div>
      <template #footer>
        <VocaButton variant="ghost" @click="deleteOpen = false">取消</VocaButton>
        <VocaButton variant="danger" @click="confirmDelete">删除角色</VocaButton>
      </template>
    </VocaDialog>
  </div>
</template>
