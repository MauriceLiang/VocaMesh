<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ChevronDown, ChevronRight, Users } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaSegmented from "@/components/ui/VocaSegmented.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import VocaInput from "@/components/ui/VocaInput.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import OrgChart from "./OrgChart.vue";
import { useTeamsStore } from "@/stores/teams";
import { useAgentsStore } from "@/stores/agents";
import { agentStateMeta } from "@/lib/status";

const router = useRouter();
const teams = useTeamsStore();
const agents = useAgentsStore();

const view = ref<"tree" | "graph">("graph");
const expanded = ref<string | null>(null);
const addChildParent = ref<string | null>(null);
const addMemberOpen = ref(false);
const addAgentId = ref("");
const editOpen = ref(false);
const editName = ref("");
const editWorkspace = ref("");
const superiorFor = ref<string | null>(null);
const removeFor = ref<string | null>(null);

const selected = computed(() => teams.selected);
const supervisorName = computed(() => teams.members.find((member) => member.agentId === selected.value?.supervisorId)?.name ?? "—");
const notReady = computed(() => teams.members.filter((member) => member.state !== "ready"));

const agentItems = computed(() => agents.agents.map((agent) => ({ value: agent.id, label: `${agent.name} · ${agent.role}` })));
const superiorItems = computed(() => [
  ...teams.members.map((member) => ({ value: member.agentId, label: member.name })),
  { value: "user", label: "用户（成为团队主管）" },
]);

onMounted(async () => {
  await Promise.all([teams.load(), agents.load()]);
  expanded.value = teams.selectedId;
});

function selectTeam(id: string): void {
  teams.select(id);
  expanded.value = id;
  editName.value = teams.selected?.name ?? "";
  editWorkspace.value = teams.selected?.workspace ?? "";
}

function openEdit(): void {
  editName.value = selected.value?.name ?? "";
  editWorkspace.value = selected.value?.workspace ?? "";
  editOpen.value = true;
}

async function saveEdit(): Promise<void> {
  await teams.updateTeam({ name: editName.value, workspace: editWorkspace.value });
  editOpen.value = false;
}

async function addMember(): Promise<void> {
  if (!addAgentId.value) return;
  const reportsTo = addChildParent.value ?? selected.value?.supervisorId ?? "user";
  const ok = await teams.addMember(addAgentId.value, reportsTo);
  if (ok) addMemberOpen.value = false;
}

async function applySuperior(agentId: string, reportsTo: string): Promise<void> {
  const ok = await teams.setReportsTo(agentId, reportsTo);
  if (ok) superiorFor.value = null;
}

async function removeMember(agentId: string): Promise<void> {
  const ok = await teams.removeMember(agentId);
  if (ok) removeFor.value = null;
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="Agent 团队" subtitle="谁向谁汇报。关系图可直接创建与调整 Agent；树形视图用于查看直属关系。">
      <template #actions>
        <VocaButton variant="primary" @click="addMemberOpen = true">新建团队</VocaButton>
      </template>
    </PageHeader>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-col-list v-card v-col" :style="{ padding: '8px', gap: '2px', overflow: 'auto' }">
        <template v-for="team in teams.teams" :key="team.id">
          <button
            class="flex items-center gap-2 rounded-[6px] px-[9px] py-[7px] text-left"
            :style="{ background: team.id === teams.selectedId ? 'var(--bg-subtle)' : 'transparent', fontWeight: team.id === teams.selectedId ? 600 : 400 }"
            @click="selectTeam(team.id)"
          >
            <component :is="expanded === team.id ? ChevronDown : ChevronRight" :size="13" class="v-muted" />
            <Users :size="13" class="v-secondary" />
            <span class="flex-1">{{ team.name }}</span>
            <span class="v-tiny v-muted">{{ team.members.length }}</span>
          </button>
          <div v-if="expanded === team.id" class="flex flex-col">
            <div v-for="member in team.members" :key="member.agentId" class="flex items-center gap-[7px] py-[5px] pl-[26px] pr-2">
              <span class="grid h-5 w-5 place-items-center rounded-[5px] text-[9.5px] font-semibold" :style="{ background: 'var(--bg-subtle)' }">{{ member.name.slice(0, 1) }}</span>
              <span class="flex min-w-0 flex-1 flex-col">
                <span class="v-small">{{ member.name }}</span>
                <span class="v-tiny v-muted">{{ member.role }}</span>
              </span>
              <VocaStatus :meta="agentStateMeta(member.state)" />
            </div>
          </div>
        </template>
      </div>

      <div class="v-grow v-card v-detail-panel">
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-[17px] font-semibold">{{ selected?.name }}</span>
          <VocaStatus :meta="{ label: teams.lastError ? '关系检查未通过' : '关系检查通过', tone: teams.lastError ? 'blocked' : 'success', icon: teams.lastError ? 'alert' : 'check' }" />
          <span class="v-spacer" />
          <VocaSegmented
            :model-value="view"
            :items="[{ value: 'tree', label: '树形' }, { value: 'graph', label: '关系图' }]"
            @update:model-value="view = $event"
          />
          <VocaButton size="sm" @click="openEdit">编辑关系</VocaButton>
          <VocaButton size="sm" variant="primary" @click="addMemberOpen = true">添加成员</VocaButton>
        </div>
        <div class="v-tiny v-muted">
          {{ teams.members.length }} 名成员 · 主管：{{ supervisorName }} · 默认工作空间 {{ selected?.workspace }} · 团队级覆盖：2 项权限
        </div>
        <div class="v-banner" :class="notReady.length ? 'is-blocked' : 'is-neutral'">
          {{ notReady.length ? `关系检查：${notReady.length} 个成员尚未就绪（${notReady.map((m) => m.name).join("、")}），任务执行到该角色时会停下并提示，不会跳过。` : "关系检查：全部成员可用。" }}
        </div>

        <OrgChart
          v-if="view === 'graph'"
          @add-child="(id) => { addChildParent = id; addMemberOpen = true; }"
          @edit="(id) => { teams.selectMember(id); editOpen = true; }"
          @superior="(id) => (superiorFor = id)"
          @make-supervisor="(id) => teams.updateTeam({ supervisorId: id })"
          @remove="(id) => (removeFor = id)"
          @new-agent="router.push({ name: 'agents' })"
        />

        <table v-else class="v-table" :style="{ border: '1px solid var(--border-default)', borderRadius: '6px', overflow: 'hidden' }">
          <thead><tr><th>成员</th><th>直属主管</th><th>执行工具</th><th>状态</th></tr></thead>
          <tbody>
            <tr v-for="member in teams.members" :key="member.agentId">
              <td>{{ member.name }}</td>
              <td class="v-small v-secondary">{{ member.reportsTo === "user" ? "用户" : teams.members.find((m) => m.agentId === member.reportsTo)?.name ?? "—" }}</td>
              <td class="v-small v-secondary">{{ member.tool }}</td>
              <td><VocaStatus :meta="agentStateMeta(member.state)" /></td>
            </tr>
          </tbody>
        </table>

        <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">
          组织关系 ≠ 任务依赖：直属主管表达汇报关系；任务中的先后依赖在任务详情的执行流程中单独表达。
        </div>
      </div>
    </div>

    <VocaDialog v-model="addMemberOpen" title="添加团队成员" subtitle="必填直属主管；若会产生自我管理或间接环，保存会被阻止。" :width="560">
      <div class="v-field">
        <span class="lbl">选择 Agent</span>
        <VocaSelect v-model="addAgentId" :items="agentItems" aria-label="选择 Agent" />
      </div>
      <div class="v-field">
        <span class="lbl">直属主管</span>
        <VocaSelect
          :model-value="addChildParent ?? selected?.supervisorId ?? 'user'"
          :items="superiorItems"
          aria-label="直属主管"
          @update:model-value="(value) => (addChildParent = value)"
        />
      </div>
      <template #footer>
        <VocaButton variant="ghost" @click="addMemberOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="addMember">添加成员</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="editOpen" title="编辑团队" subtitle="团队级覆盖只影响新任务，不会改变正在运行的 Run。" :width="560">
      <div class="v-field"><span class="lbl">团队名称</span><VocaInput v-model="editName" aria-label="团队名称" /></div>
      <div class="v-field"><span class="lbl">默认工作空间</span><VocaInput v-model="editWorkspace" aria-label="默认工作空间" /></div>
      <template #footer>
        <VocaButton variant="ghost" @click="editOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="saveEdit">保存</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog :model-value="!!superiorFor" title="设置直属上级" subtitle="禁止自我管理与间接环；保存前会校验。" :width="560" @update:model-value="superiorFor = null">
      <div class="flex flex-col gap-[6px]">
        <button
          v-for="item in superiorItems"
          :key="item.value"
          class="flex items-center gap-[9px] rounded-[6px] border px-[10px] py-2 text-left"
          :style="{ borderColor: 'var(--border-default)' }"
          @click="superiorFor && applySuperior(superiorFor, item.value)"
        >
          <span class="v-small">{{ item.label }}</span>
        </button>
      </div>
      <template #footer>
        <VocaButton variant="ghost" @click="superiorFor = null">取消</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog :model-value="!!removeFor" title="移除团队成员" subtitle="历史 Run 与审批记录不会删除。" :width="520" @update:model-value="removeFor = null">
      <div class="v-banner is-blocked">主管不能直接移除；如需调整请先指定新的主管。</div>
      <template #footer>
        <VocaButton variant="ghost" @click="removeFor = null">取消</VocaButton>
        <VocaButton variant="danger" @click="removeFor && removeMember(removeFor)">移除成员</VocaButton>
      </template>
    </VocaDialog>
  </div>
</template>
