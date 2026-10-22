<script setup lang="ts">
import { onMounted } from "vue";
import { FileText } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import VocaDropdown from "@/components/ui/VocaDropdown.vue";
import { useRunsStore } from "@/stores/runs";
import { useTasksStore } from "@/stores/tasks";
import { runStateMeta } from "@/lib/status";
import { useAppStore } from "@/stores/app";

const runs = useRunsStore();
const tasks = useTasksStore();
const app = useAppStore();

const resultItems = (app.isMock ? ["全部", "已完成", "失败", "部分完成", "已停止", "状态未知"] : ["全部", "已完成", "失败", "已停止", "状态未知"]).map((value) => ({ label: value, value }));
const typeItems = ["全部 Run", "仅失败", "仅审批相关"].map((value) => ({ label: value, value }));

onMounted(async () => {
  await runs.load();
});

async function retry(): Promise<void> {
  const first = tasks.tasks[0];
  if (first) await tasks.retryFailed(first.id);
  await runs.load();
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="历史记录" :subtitle="app.isMock ? '追溯每一次 Run 与关键操作。重试会创建新的 Run，旧日志与产物保持不变。' : '本机 Claude Code Run 历史。暂停、恢复、重试和文件产物尚未实现。'">
      <template #actions><VocaButton size="sm" @click="runs.exportCsv"><FileText :size="12" />导出记录 (CSV)</VocaButton></template>
    </PageHeader>

    <div class="flex flex-wrap items-center gap-2">
      <VocaDropdown v-if="app.isMock" :items="typeItems" @select="(value) => { runs.resultFilter = value === '仅失败' ? '失败' : '全部'; runs.load(); }">
        <VocaButton size="sm">类型：{{ runs.resultFilter === "失败" ? "仅失败" : "全部 Run" }}</VocaButton>
      </VocaDropdown>
      <VocaDropdown :items="resultItems" @select="(value) => { runs.resultFilter = value; runs.load(); }">
        <VocaButton size="sm">结果：{{ runs.resultFilter }}</VocaButton>
      </VocaDropdown>
      <VocaButton size="sm" variant="ghost" @click="runs.resultFilter = '全部'; runs.load()">清除筛选</VocaButton>
    </div>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-grow v-table-wrap" :style="{ overflow: 'auto', minWidth: 0 }">
        <table class="v-table">
          <thead><tr><th>Run</th><th>任务与执行者</th><th>结果</th><th>开始</th><th>耗时</th></tr></thead>
          <tbody>
            <tr v-for="run in runs.runs" :key="run.id" class="clickable" :class="{ 'is-selected': run.id === runs.selectedId }" @click="runs.select(run.id)">
              <td class="v-mono v-small">{{ run.id }}</td>
              <td>{{ run.taskName }}<div class="v-tiny v-muted">{{ run.executor }}</div></td>
              <td><VocaStatus :meta="runStateMeta(run.state)" /></td>
              <td class="v-mono v-tiny v-muted">{{ run.startedAt }}</td>
              <td class="v-mono v-tiny v-muted">{{ run.duration }}</td>
            </tr>
          </tbody>
        </table>
        <div class="flex justify-center p-[10px]">
          <VocaButton size="sm" variant="ghost" :disabled="!runs.hasMore" @click="runs.loadMore()">加载更多</VocaButton>
        </div>
      </div>

      <div class="v-col-rail v-card v-detail-panel">
        <template v-if="app.isMock">
        <div class="flex items-center gap-[7px]"><b class="v-mono text-[13px]">{{ runs.selected?.id ?? "#1244" }}</b><VocaStatus :meta="{ label: '部分完成', tone: 'blocked', icon: 'alert' }" /></div>
        <div class="v-small v-secondary">批量规范化 320 条术语词表</div>
        <div class="v-banner is-blocked" :style="{ alignItems: 'flex-start' }">
          部分成果：5 个子任务中 3 项完成、1 项跳过、1 项失败。整体不算成功。
        </div>
        <div v-for="group in runs.partialGroups" :key="group.label" class="flex flex-col gap-[5px]">
          <div class="flex items-center gap-[6px]">
            <VocaStatus :meta="{ label: group.label, tone: group.kind, icon: group.kind === 'success' ? 'check' : group.kind === 'failure' ? 'x' : 'dot' }" />
            <span class="v-tiny v-muted">{{ group.items.length }} 项</span>
          </div>
          <div v-for="item in group.items" :key="item.path" class="flex items-center gap-[7px] rounded-[4px] px-2 py-[6px]" :style="{ background: 'var(--bg-inset)' }">
            <FileText :size="11" class="v-muted" />
            <span class="v-mono v-tiny flex-1">{{ item.path }}</span>
            <span class="v-tiny v-muted">{{ item.note }}</span>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <VocaButton size="sm" :disabled="!app.isMock" @click="runs.exportPartial">导出部分成果</VocaButton>
          <VocaButton size="sm" variant="primary" @click="retry">在新 Run 中重试</VocaButton>
        </div>
        <div class="v-tiny v-muted">重试将创建新 Run，并保留旧 Run 的全部日志与产物。</div>
        </template>
        <template v-else-if="runs.selected">
          <div class="flex items-center gap-[7px]"><b class="v-mono text-[13px]">{{ runs.selected.id }}</b><VocaStatus :meta="runStateMeta(runs.selected.state)" /></div>
          <div class="v-small v-secondary">{{ runs.selected.taskName }} · {{ runs.selected.executor }}</div>
          <div v-if="runs.selected.outcome" class="v-tiny v-muted">结果：{{ runs.selected.outcome }}<span v-if="runs.selected.finishedAt"> · {{ runs.selected.finishedAt }}</span></div>
          <div v-if="runs.selected.errorCode" class="v-tiny v-muted">错误码：{{ runs.selected.errorCode }}</div>
          <div v-if="runs.selected.summary" class="v-card v-panel whitespace-pre-wrap text-[11.5px]">{{ runs.selected.summary }}</div>
          <div v-if="runs.selected.outputTruncated" class="v-banner is-blocked">输出已达到本 Run 的保存上限。</div>
          <div class="v-card v-panel"><span class="v-small">本阶段只读，不生成文件产物。</span><span class="v-tiny v-muted">日志通过任务详情的检查器查看。</span></div>
          <VocaButton size="sm" variant="primary" disabled>重试（本阶段未实现）</VocaButton>
        </template>
        <div v-else class="v-empty"><span class="v-small">暂无本机 Run 历史</span></div>
      </div>
    </div>
  </div>
</template>
