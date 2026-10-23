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
import { useWorkspacesStore } from "@/stores/workspaces";
import { agentStateMeta } from "@/lib/status";
import { useServices } from "@/services/adapters";

const router = useRouter();
const teams = useTeamsStore();
const agents = useAgentsStore();
const workspaces = useWorkspacesStore();
const services = useServices();
const isNative = services.mode === "tauri";

const view = ref<"tree" | "graph">("graph");
const expanded = ref<string | null>(null);
const addChildParent = ref<string | null>(null);
const addMemberOpen = ref(false);
const addAgentId = ref("");
const createTeamOpen = ref(false);
const createTeamName = ref("");
const createSupervisorId = ref("");
const createWorkspaceId = ref("");
const editOpen = ref(false);
const editName = ref("");
const editWorkspace = ref("");
const editSupervisorId = ref("");
const superiorFor = ref<string | null>(null);
const removeFor = ref<string | null>(null);

const selected = computed(() => teams.selected);
const supervisorName = computed(() => teams.members.find((member) => member.agentId === selected.value?.supervisorId)?.name ?? "—");
const notReady = computed(() => teams.members.filter((member) => member.state !== "ready"));

const agentItems = computed(() => agents.agents.filter((agent) => !selected.value?.members.some((member) => member.agentId === agent.id)).map((agent) => ({ value: agent.id, label: `${agent.name} · ${agent.role}` })));
const superiorItems = computed(() => [
  ...teams.members.map((member) => ({ value: member.agentId, label: member.name })),
  { value: "user", label: "用户（成为团队主管）" },
]);
const supervisorItems = computed(() => teams.members.map((member) => ({ value: member.agentId, label: `${member.name} · ${member.role}` })));
const addSuperiorItems = computed(() => teams.members.filter((member) => member.agentId !== addAgentId.value).map((member) => ({ value: member.agentId, label: member.name })));
const workspaceItems = computed(() => workspaces.workspaces.map((workspace) => ({
  value: isNative ? workspace.id : workspace.name,
  label: `${workspace.name} · ${workspace.path}${isNative && !workspace.pathAvailable ? "（目录需检查）" : ""}`,
})));
const validWorkspaceItems = computed(() => workspaceItems.value.filter((item) => !isNative || workspaces.workspaces.find((workspace) => workspace.id === item.value)?.pathAvailable));
const selectedWorkspaceNeedsCheck = computed(() => isNative && Boolean(selected.value) && !workspaces.workspaces.find((workspace) => workspace.id === selected.value?.workspaceId)?.pathAvailable);
const createAgentItems = computed(() => agents.agents.map((agent) => ({ value: agent.id, label: `${agent.name} · ${agent.role}` })));

onMounted(async () => {
  await Promise.all([teams.load(), agents.load(), workspaces.load()]);
  expanded.value = teams.selectedId;
  createSupervisorId.value = agents.agents[0]?.id ?? "";
  createWorkspaceId.value = validWorkspaceItems.value[0]?.value ?? "";
});

function selectTeam(id: string): void {
  teams.select(id);
  expanded.value = id;
  editName.value = teams.selected?.name ?? "";
  editWorkspace.value = teams.selected?.workspaceId ?? teams.selected?.workspace ?? "";
  editSupervisorId.value = teams.selected?.supervisorId ?? "";
}

function openEdit(): void {
  editName.value = selected.value?.name ?? "";
  editWorkspace.value = selected.value?.workspaceId ?? selected.value?.workspace ?? "";
  editSupervisorId.value = selected.value?.supervisorId ?? "";
  editOpen.value = true;
}

async function saveEdit(): Promise<void> {
  const ok = await teams.updateTeam(isNative
    ? { name: editName.value, supervisorId: editSupervisorId.value, workspaceId: editWorkspace.value }
    : { name: editName.value, supervisorId: editSupervisorId.value, workspace: editWorkspace.value });
  if (ok) editOpen.value = false;
}

async function addMember(): Promise<void> {
  if (!addAgentId.value) return;
  const reportsTo = addChildParent.value ?? selected.value?.supervisorId ?? "user";
  const ok = await teams.addMember(addAgentId.value, reportsTo);
  if (ok) addMemberOpen.value = false;
}

async function createTeam(): Promise<void> {
  if (!createTeamName.value.trim() || !createSupervisorId.value || !createWorkspaceId.value) return;
  const team = await teams.createTeam({
    name: createTeamName.value.trim(),
    supervisorId: createSupervisorId.value,
    workspace: isNative ? createWorkspaceId.value : workspaces.workspaces.find((workspace) => workspace.name === createWorkspaceId.value)?.name ?? createWorkspaceId.value,
    workspaceId: isNative ? createWorkspaceId.value : undefined,
  });
  if (team) {
    createTeamOpen.value = false;
    createTeamName.value = "";
  }
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
    <PageHeader title="Agent 团队" :subtitle="isNative ? '本机保存团队与汇报关系；团队配置不会启动多 Agent 执行。' : '谁向谁汇报。关系图可直接创建与调整 Agent；树形视图用于查看直属关系。'">
      <template #actions>
        <VocaButton variant="primary" @click="createTeamOpen = true">新建团队</VocaButton>
      </template>
    </PageHeader>
    <div v-if="teams.lastError" class="v-banner is-failure"><span class="flex-1">{{ teams.lastError }}</span><VocaButton size="sm" @click="teams.load()">重试</VocaButton></div>
    <div v-if="agents.lastError" class="v-banner is-failure"><span class="flex-1">{{ agents.lastError }}</span><VocaButton size="sm" @click="agents.load()">重试 Agent</VocaButton></div>
    <div v-if="workspaces.lastError" class="v-banner is-failure"><span class="flex-1">{{ workspaces.lastError }}</span><VocaButton size="sm" @click="workspaces.load()">重试工作空间</VocaButton></div>
    <div v-if="selectedWorkspaceNeedsCheck" class="v-banner is-blocked">该团队登记的目录当前不可用。请检查工作空间路径，或在编辑团队时改选一个有效目录。</div>

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

      <div v-if="selected" class="v-grow v-card v-detail-panel">
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
          {{ teams.members.length }} 名成员 · 主管：{{ supervisorName }} · 默认工作空间 {{ selected.workspace }} · {{ isNative ? "团队关系仅用于管理配置，不会启动任务" : "团队级覆盖：2 项权限" }}
        </div>
        <div class="v-banner" :class="notReady.length ? 'is-blocked' : 'is-neutral'">
          {{ isNative ? "团队关系已保存；Agent 角色不会因此获得执行能力，真实执行仍受安全策略阻断。" : notReady.length ? `关系检查：${notReady.length} 个成员尚未就绪（${notReady.map((m) => m.name).join("、")}），任务执行到该角色时会停下并提示，不会跳过。` : "关系检查：全部成员可用。" }}
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
        <div v-if="isNative" class="v-banner is-neutral">关系图节点位置只保存在本次界面会话，重新打开后可能重新布局。</div>
      </div>
      <div v-else class="v-grow v-card v-empty">
        <Users :size="18" class="v-muted" />
        <span class="text-[15px] font-semibold">{{ teams.lastError ? "无法读取本机团队配置" : "尚无团队" }}</span>
        <span class="v-small">{{ teams.lastError ?? (agents.agents.length && workspaces.workspaces.length ? "创建团队前需选择真实主管 Agent 和已登记工作空间。" : "请先添加 Agent 角色和工作空间，然后再创建团队。") }}</span>
        <div class="flex gap-2">
          <VocaButton v-if="teams.lastError" size="sm" @click="teams.load()">重试</VocaButton>
          <VocaButton v-else-if="!agents.agents.length" size="sm" @click="router.push({ name: 'agents' })">添加 Agent</VocaButton>
          <VocaButton v-else-if="!workspaces.workspaces.length" size="sm" @click="router.push({ name: 'workspaces' })">添加工作空间</VocaButton>
          <VocaButton v-else size="sm" variant="primary" @click="createTeamOpen = true">新建团队</VocaButton>
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
          :items="addSuperiorItems"
          aria-label="直属主管"
          @update:model-value="(value) => (addChildParent = value)"
        />
      </div>
      <template #footer>
        <VocaButton variant="ghost" @click="addMemberOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="addMember">添加成员</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="createTeamOpen" title="新建团队" :subtitle="isNative ? '主管和默认工作空间都必须来自本机已保存配置。' : '团队关系用于组织结构；Mock 模式不会触发真实执行。'" :width="560">
      <div class="v-field"><span class="lbl">团队名称</span><VocaInput v-model="createTeamName" aria-label="团队名称" /></div>
      <div class="v-field"><span class="lbl">主管 Agent</span><VocaSelect v-model="createSupervisorId" :items="createAgentItems" aria-label="主管 Agent" /></div>
      <div class="v-field"><span class="lbl">默认工作空间</span><VocaSelect v-model="createWorkspaceId" :items="validWorkspaceItems" aria-label="默认工作空间" /></div>
      <div v-if="!createAgentItems.length || !validWorkspaceItems.length" class="v-banner is-neutral">请先登记一个 Agent 角色和一个有效的工作空间目录。</div>
      <template #footer>
        <VocaButton variant="ghost" @click="createTeamOpen = false">取消</VocaButton>
        <VocaButton variant="primary" :disabled="!createAgentItems.length || !validWorkspaceItems.length" @click="createTeam">创建团队</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="editOpen" title="编辑团队" :subtitle="isNative ? '主管必须是当前团队成员；本机只保存组织关系，不会启动任务。' : '团队级覆盖只影响新任务，不会改变正在运行的 Run。'" :width="560">
      <div class="v-field"><span class="lbl">团队名称</span><VocaInput v-model="editName" aria-label="团队名称" /></div>
      <div class="v-field"><span class="lbl">主管 Agent</span><VocaSelect v-model="editSupervisorId" :items="supervisorItems" aria-label="主管 Agent" /></div>
      <div class="v-field"><span class="lbl">默认工作空间</span><VocaSelect v-model="editWorkspace" :items="workspaceItems" aria-label="默认工作空间" /></div>
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
