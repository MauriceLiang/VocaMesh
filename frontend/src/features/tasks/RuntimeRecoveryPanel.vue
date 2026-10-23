<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { AlertTriangle, RefreshCw } from "lucide-vue-next";
import VocaButton from "@/components/ui/VocaButton.vue";
import { useServices } from "@/services/adapters";
import type { RecoveryInspection, UnknownRunSummary } from "@/types";

const props = defineProps<{ runId?: string }>();
const services = useServices();
const entries = ref<UnknownRunSummary[]>([]);
const inspections = ref<Record<string, RecoveryInspection>>({});
const busyRunId = ref<string | null>(null);
const error = ref<string | null>(null);
const notice = ref<string | null>(null);
const loading = ref(false);

const visibleEntries = computed(() => props.runId
  ? entries.value.filter((entry) => entry.run.id === props.runId)
  : entries.value);

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    entries.value = await services.execution.listUnknownRuns();
  } catch {
    error.value = "无法读取未确认的运行记录；本地历史仍保留，新任务继续暂停。";
  } finally {
    loading.value = false;
  }
}

async function inspect(runId: string): Promise<void> {
  busyRunId.value = runId;
  error.value = null;
  notice.value = null;
  try {
    const result = await services.execution.inspectUnknownRun(runId);
    inspections.value = { ...inspections.value, [runId]: result };
    entries.value = entries.value.map((entry) => entry.run.id === runId
      ? { ...entry, snapshotStatus: result.snapshotStatus }
      : entry);
  } catch {
    error.value = "检查失败；运行记录与快照保持不变，请稍后重试。";
  } finally {
    busyRunId.value = null;
  }
}

async function resolve(runId: string): Promise<void> {
  const confirmed = window.confirm(
    "请确认你已在系统进程管理工具中检查 VocaMesh、Claude Code 及相关子进程，并确认没有仍在运行的相关进程。进程组检查无法发现已脱离原进程组的后代。归档会将该 Run 标记为失败，不会重跑任务。",
  );
  if (!confirmed) return;
  busyRunId.value = runId;
  error.value = null;
  notice.value = null;
  try {
    const resolution = await services.execution.resolveUnknownRun(runId, true);
    entries.value = entries.value.filter((entry) => entry.run.id !== runId);
    const next = { ...inspections.value };
    delete next[runId];
    inspections.value = next;
    notice.value = resolution.snapshotStatus === "missing"
      ? "Run 已归档为失败，快照已清理；任务没有自动重跑。"
      : "Run 已归档为失败，但快照清理未能确认；已记录诊断，请检查本地应用数据。任务没有自动重跑。";
    await load();
  } catch {
    await load();
    error.value = "归档未完成或未能确认；已重新读取后端状态，仍未知的运行记录会继续显示并阻止新任务。";
  } finally {
    busyRunId.value = null;
  }
}

function statusLabel(entry: UnknownRunSummary): string {
  const status = inspections.value[entry.run.id]?.processGroupStatus;
  if (status === "exited-candidate") return "已确认进程组退出（仅为人工归档候选）";
  if (status === "still-running") return "检测到进程组仍存在";
  if (status === "manual-required") return "需要人工处理";
  if (status === "cannot-verify") return "无法核实进程状态";
  return entry.run.recoveryStatus === "manual-required" ? "需要人工处理" : "等待检查";
}

function snapshotLabel(status: UnknownRunSummary["snapshotStatus"]): string {
  if (status === "exists") return "存在";
  if (status === "missing") return "不存在";
  return "无法检查";
}

function reasonExplanation(errorCode: string | null): string {
  if (errorCode === "run.stop-unconfirmed") return "运行时无法确认相关进程树已退出，因此保留了记录和快照。";
  if (errorCode === "storage.write-failed") return "本地历史写入未能确认，运行结果保持未知。";
  if (errorCode === "execution.process-status-unknown") return "操作系统未能确认进程状态，归档前需要再次核验。";
  return "任务执行未能安全确认结束；原始错误原因保留在 Run 历史中。";
}

function lastChecked(entry: UnknownRunSummary): string {
  const timestamp = inspections.value[entry.run.id]?.run.recoveryCheckedAt ?? entry.run.recoveryCheckedAt;
  return timestamp == null ? "尚未检查" : new Date(timestamp).toLocaleString();
}

onMounted(() => { void load(); });
</script>

<template>
  <section v-if="visibleEntries.length || loading || error || notice" class="v-card v-panel" aria-label="运行状态恢复">
    <div class="flex items-start gap-2">
      <AlertTriangle :size="14" class="mt-0.5" :style="{ color: 'var(--status-blocked)' }" />
      <div class="min-w-0 flex-1">
        <div class="v-small font-semibold">运行状态需要确认</div>
        <p class="v-tiny v-muted mt-1">为避免重复执行，VocaMesh 暂停了新的真实任务。恢复不会证明原任务成功，也不会自动重跑。</p>
      </div>
      <VocaButton size="sm" variant="ghost" :disabled="loading" aria-label="重新加载恢复记录" @click="load">
        <RefreshCw :size="13" />
      </VocaButton>
    </div>

    <div v-if="error" role="alert" class="v-banner is-failure"><span class="v-tiny">{{ error }}</span></div>
    <div v-if="notice" role="status" class="v-banner is-neutral"><span class="v-tiny">{{ notice }}</span></div>
    <div v-if="loading && !visibleEntries.length" class="v-tiny v-muted">正在读取恢复记录…</div>

    <article v-for="entry in visibleEntries" :key="entry.run.id" class="rounded-md border p-3" :style="{ borderColor: 'var(--border-default)' }">
      <div class="flex flex-wrap items-start justify-between gap-2">
        <div class="min-w-0">
          <div class="v-small font-medium">{{ entry.taskTitle }}</div>
          <div class="v-tiny v-muted mt-1">Run …{{ entry.run.id.slice(-8) }} · {{ new Date(entry.run.finishedAt ?? entry.run.startedAt ?? Date.now()).toLocaleString() }}</div>
        </div>
        <span class="v-tiny">{{ statusLabel(entry) }}</span>
      </div>
      <div class="mt-2 grid gap-1 v-tiny v-muted">
        <span>错误码：{{ entry.run.errorCode ?? "未记录" }}</span>
        <span>{{ reasonExplanation(entry.run.errorCode) }}</span>
        <span>快照：{{ snapshotLabel(entry.snapshotStatus) }}</span>
        <span>最近检查：{{ lastChecked(entry) }}</span>
      </div>
      <div class="mt-3 flex flex-wrap gap-2">
        <VocaButton size="sm" variant="ghost" :disabled="busyRunId === entry.run.id" @click="inspect(entry.run.id)">
          {{ busyRunId === entry.run.id ? "正在检查…" : "重新检查" }}
        </VocaButton>
        <VocaButton
          v-if="inspections[entry.run.id]?.processGroupStatus === 'exited-candidate' && inspections[entry.run.id]?.run.recoveryStatus === 'required'"
          size="sm"
          variant="primary"
          :disabled="busyRunId === entry.run.id"
          @click="resolve(entry.run.id)"
        >
          我已人工核验并归档
        </VocaButton>
      </div>
      <details class="mt-3 v-tiny v-muted">
        <summary class="cursor-pointer">查看恢复说明</summary>
        <p class="mt-2">请在系统进程管理工具中检查 VocaMesh、Claude Code 及相关子进程。VocaMesh 不能证明已脱离原进程组的后代不存在，所以进程组退出只提供候选证据，不会自动解锁。不要仅凭 PID 结束进程或删除快照。</p>
      </details>
    </article>

    <div v-if="!loading && !error && !visibleEntries.length" class="v-tiny v-muted">没有需要处理的 Unknown Run。</div>
  </section>
</template>
