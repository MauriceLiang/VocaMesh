<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { AlertTriangle, FolderOpen } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import VocaInput from "@/components/ui/VocaInput.vue";
import { useWorkspacesStore } from "@/stores/workspaces";
import { useTasksStore } from "@/stores/tasks";
import { useAppStore } from "@/stores/app";
import { useTeamsStore } from "@/stores/teams";
import { taskStateMeta } from "@/lib/status";
import { useServices } from "@/services/adapters";

const workspaces = useWorkspacesStore();
const tasks = useTasksStore();
const app = useAppStore();
const teams = useTeamsStore();
const services = useServices();
const isNative = services.mode === "tauri";

const createOpen = ref(false);
const renameOpen = ref(false);
const archiveOpen = ref(false);
const newName = ref("");
const newPath = ref("");
const renameValue = ref("");
const boundaryOpen = ref(false);
const readValue = ref("");
const writeValue = ref("");

const selected = computed(() => workspaces.selected);
const related = computed(() => tasks.tasks.filter((task) => task.workspace === selected.value?.path || task.workspace === selected.value?.name));
const teamsUsing = computed(() => teams.teams.filter((team) => team.workspaceId === selected.value?.id || team.workspace === selected.value?.name).map((team) => team.name));

onMounted(async () => {
  await Promise.all([workspaces.load(), tasks.load(), teams.load()]);
});

function openBoundary(): void {
  readValue.value = selected.value?.read.join(", ") ?? "";
  writeValue.value = selected.value?.write.join(", ") ?? "";
  boundaryOpen.value = true;
}

async function saveBoundary(): Promise<void> {
  const parse = (value: string) => value.split(",").map((item) => item.trim()).filter(Boolean);
  const ok = await workspaces.updateBoundaries(parse(readValue.value), parse(writeValue.value));
  if (ok) boundaryOpen.value = false;
}

async function createWorkspace(): Promise<void> {
  const ok = await workspaces.create(newName.value, newPath.value);
  if (ok) {
    createOpen.value = false;
    newName.value = "";
    newPath.value = "";
  }
}

function openRename(): void {
  renameValue.value = selected.value?.name ?? "";
  renameOpen.value = true;
}

async function renameWorkspace(): Promise<void> {
  if (await workspaces.rename(renameValue.value)) renameOpen.value = false;
}

async function chooseDirectory(): Promise<void> {
  try {
    const path = await services.execution.selectWorkspaceDirectory();
    if (path) newPath.value = path;
  } catch {
    app.toast("无法打开系统目录选择器，请重试。", "failure");
  }
}

async function copyPath(): Promise<void> {
  const path = await workspaces.resolveOpenPath();
  void navigator.clipboard?.writeText(path).then(
    () => app.toast(`已复制路径：${path}`, "success"),
    () => app.toast("当前环境不允许写入剪贴板", "blocked"),
  );
}

async function archiveWorkspace(): Promise<void> {
  if (!selected.value) return;
  if (await workspaces.archive(selected.value.id)) archiveOpen.value = false;
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="工作空间" :subtitle="isNative ? '本地目录登记与范围草稿；登记不会授予 Agent 文件访问权限。' : '任务能读写的本地目录范围。边界在这里定义，Agent 只能看到被授权的部分。'">
      <template #actions><VocaButton variant="primary" @click="createOpen = true">添加工作空间</VocaButton></template>
    </PageHeader>
    <div v-if="workspaces.lastError" class="v-banner is-failure"><span class="flex-1">{{ workspaces.lastError }}</span><VocaButton size="sm" @click="workspaces.load()">重试</VocaButton></div>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-col-list v-col" :style="{ gap: '8px', overflow: 'auto' }">
        <button
          v-for="item in workspaces.workspaces"
          :key="item.id"
          class="v-card flex flex-col gap-[6px] text-left"
          :style="{ padding: workspaces.selectedId === item.id ? '11px' : '12px', border: workspaces.selectedId === item.id ? '2px solid var(--text-primary)' : '1px solid var(--border-default)' }"
          @click="workspaces.select(item.id)"
        >
          <span class="flex items-center gap-[7px]">
            <FolderOpen :size="13" class="v-secondary" />
            <span class="v-mono flex-1" :style="{ fontSize: '11.5px' }">{{ item.name }}</span>
            <VocaStatus :meta="isNative ? item.pathAvailable ? { label: '已登记 · 未授权', tone: 'neutral', icon: 'dot' } : { label: '目录需检查', tone: 'blocked', icon: 'alert' } : item.state === 'conflict' ? { label: '2 任务共用', tone: 'blocked', icon: 'alert' } : item.state === 'readonly' ? { label: '只读挂载', tone: 'neutral', icon: 'dot' } : { label: '可用', tone: 'success', icon: 'check' }" />
          </span>
          <span class="flex items-center gap-2"><span class="v-mono v-tiny v-muted">{{ item.path }}</span><span class="v-tiny v-muted">· {{ item.taskCount }} 个任务</span></span>
        </button>
      </div>

      <div v-if="selected" class="v-grow v-card v-detail-panel">
        <div class="flex flex-wrap items-center gap-2">
          <span class="v-mono text-[16px] font-semibold">{{ selected.name }}</span>
          <span class="v-mono v-tiny v-muted">{{ selected.path }}</span>
          <span class="v-spacer" />
          <VocaButton size="sm" @click="copyPath">复制路径</VocaButton>
          <VocaButton size="sm" @click="openRename">重命名</VocaButton>
          <VocaButton size="sm" variant="primary" @click="openBoundary">编辑边界</VocaButton>
          <VocaButton v-if="isNative" size="sm" variant="danger" @click="archiveOpen = true">归档工作空间</VocaButton>
        </div>

        <div v-if="isNative && !selected.pathAvailable" class="v-banner is-blocked" :style="{ alignItems: 'flex-start' }">
          <AlertTriangle :size="13" />
          <span class="v-small">登记的目录当前不存在、无法访问或不再符合安全路径规则。目录关联仍保留在本机数据库中；检查目录后可重新登记。</span>
        </div>

        <div v-if="!isNative && selected.state === 'conflict'" class="v-banner is-blocked" :style="{ alignItems: 'flex-start' }">
          <AlertTriangle :size="13" />
          <span class="flex flex-1 flex-col gap-[3px]">
            <b class="v-small">检测到文件冲突风险：2 个任务正在写入同一目录</b>
            <span class="v-tiny" :style="{ lineHeight: 1.5 }">Run #1244 与 Run #1240 都会写入 terms/。系统不会假定已实现隔离，请选择处理方式。</span>
          </span>
          <VocaButton size="sm" @click="workspaces.resolveConflict()">排队其中一个</VocaButton>
        </div>
        <div v-else-if="!isNative && selected.conflictInfo" class="v-banner is-neutral">{{ selected.conflictInfo }} · 冲突已缓解</div>

        <div class="grid grid-cols-3 gap-[10px]">
          <div class="v-card v-panel" :style="{ background: 'var(--bg-inset)', gap: '4px' }">
            <b class="v-small">{{ isNative ? '读取范围草稿' : '可读取' }}</b>
            <span class="v-mono v-tiny v-muted">{{ selected.read.join(" · ") || "—" }}</span>
          </div>
          <div class="v-card v-panel" :style="{ background: 'var(--bg-inset)', gap: '4px' }">
            <b class="v-small">{{ isNative ? '未来写入范围（未启用）' : '可写入' }}</b>
            <span class="v-mono v-tiny v-muted">{{ selected.write.join(" · ") || "无" }}</span>
          </div>
          <div class="v-card v-panel" :style="{ background: 'var(--bg-inset)', gap: '4px' }">
            <b class="v-small">禁止访问</b>
            <span class="v-mono v-tiny v-muted">{{ selected.deny.join(" · ") }}</span>
          </div>
        </div>

        <div class="flex items-start gap-3">
          <div class="flex flex-1 flex-col gap-[6px]">
            <span class="v-section-title">相关任务</span>
            <button v-for="task in related" :key="task.id" class="flex items-center gap-2 rounded-[5px] px-[9px] py-[7px] text-left" :style="{ background: 'var(--bg-inset)' }">
              <span class="v-small flex-1">{{ task.name }}</span>
              <VocaStatus :meta="taskStateMeta(task.state)" />
            </button>
            <span v-if="!related.length" class="v-tiny v-muted">此工作空间暂无任务。</span>
          </div>
          <div class="flex w-[260px] flex-col gap-[6px]">
            <span class="v-section-title">使用此空间的团队</span>
            <span class="v-tiny v-muted">{{ teamsUsing.length ? teamsUsing.join("、") : "尚无团队使用此空间。" }}</span>
          </div>
        </div>
      </div>
      <div v-else-if="workspaces.lastError" class="v-grow v-card v-empty">
        <span class="text-[15px] font-semibold">无法读取本机工作空间配置</span>
        <span class="v-small">{{ workspaces.lastError }}</span>
        <VocaButton size="sm" @click="workspaces.load()">重试</VocaButton>
      </div>
      <div v-else-if="!workspaces.loading && !workspaces.workspaces.length" class="v-grow v-card v-empty">
        <FolderOpen :size="18" class="v-muted" />
        <span class="text-[15px] font-semibold">尚未登记工作空间</span>
        <span class="v-small">添加一个本机目录后，可在配置中选择它。登记不会授予 Runtime 文件权限。</span>
        <VocaButton size="sm" variant="primary" @click="createOpen = true">添加工作空间</VocaButton>
      </div>
    </div>

    <VocaDialog v-model="createOpen" title="添加工作空间" :subtitle="isNative ? '选择真实目录；Rust 会在保存前执行安全校验。' : '写入范围默认关闭，需要显式开启。'" :width="560">
      <div class="v-field"><span class="lbl">名称</span><VocaInput v-model="newName" aria-label="工作空间名称" /></div>
      <div class="v-field"><span class="lbl">本地路径</span><div class="flex gap-2"><VocaInput v-model="newPath" aria-label="本地路径" /><VocaButton v-if="isNative" @click="chooseDirectory">选择目录</VocaButton></div></div>
      <div v-if="isNative" class="v-banner is-neutral">目录登记不会授予 Runtime 读写权限。你也可以手动输入路径，但会执行相同的后端校验。</div>
      <template #footer>
        <VocaButton variant="ghost" @click="createOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="createWorkspace">添加工作空间</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="renameOpen" title="重命名工作空间" subtitle="团队通过稳定 ID 关联此工作空间；改名不会断开关系。" :width="520">
      <div class="v-field"><span class="lbl">工作空间名称</span><VocaInput v-model="renameValue" aria-label="工作空间名称" /></div>
      <template #footer>
        <VocaButton variant="ghost" @click="renameOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="renameWorkspace">保存名称</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="archiveOpen" title="归档工作空间" subtitle="归档会从本机目录列表中隐藏它，不会删除目录或历史任务。仍被团队引用时，后端会拒绝归档。" :width="520">
      <div v-if="selected" class="v-banner is-neutral">{{ selected.name }} · {{ selected.path }}</div>
      <template #footer>
        <VocaButton variant="ghost" @click="archiveOpen = false">取消</VocaButton>
        <VocaButton variant="danger" @click="archiveWorkspace">归档</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="boundaryOpen" title="编辑访问边界" :subtitle="isNative ? '本地保存范围草稿；当前不会授予 Runtime 文件权限。' : '逗号分隔多个路径；禁止清单始终生效。'" :width="620">
      <div class="v-field"><span class="lbl">可读取路径</span><VocaInput v-model="readValue" aria-label="可读取路径" /></div>
      <div class="v-field"><span class="lbl">未来可写入路径（尚未启用）</span><VocaInput v-model="writeValue" aria-label="未来可写入路径" :disabled="isNative" /></div>
      <div class="v-banner is-neutral" :style="{ alignItems: 'flex-start' }">{{ isNative ? '范围仅作为配置草稿；Runtime 不会把它们当作授权。' : '扩大写入范围会应用到下一个新任务；正在运行的 Run 仍使用旧边界。' }}</div>
      <template #footer>
        <VocaButton variant="ghost" @click="boundaryOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="saveBoundary">保存边界</VocaButton>
      </template>
    </VocaDialog>
  </div>
</template>
