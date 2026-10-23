<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { ArrowUpRight, Shield } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import { useApprovalsStore } from "@/stores/approvals";
import { approvalStateMeta } from "@/lib/status";
import { useServices } from "@/services/adapters";

const router = useRouter();
const approvals = useApprovalsStore();
const services = useServices();
const isNative = services.mode === "tauri";

const selected = computed(() => approvals.selected);

onMounted(async () => {
  if (!approvals.approvals.length) await approvals.load();
});
</script>

<template>
  <div class="v-content">
    <PageHeader title="待审批" :subtitle="isNative ? '真实审批引擎尚未接入；当前不会生成或执行写入、命令等审批请求。' : '集中处理 Agent 等待你决定的请求。任务内的审批卡与本页指向同一条记录，不会产生两份。'">
      <template #actions><span v-if="!isNative" class="v-tiny v-muted">默认不允许，需要你明确选择</span></template>
    </PageHeader>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-col-list v-col" :style="{ gap: '8px', overflow: 'auto' }">
        <div class="flex items-center gap-[6px]">
          <button
            v-for="item in [{ value: 'pending', label: `待处理 ${approvals.pending.length}` }, { value: 'all', label: '已处理' }] as const"
            :key="item.value"
            class="v-badge"
            :style="{ height: '24px', fontSize: '11px', background: approvals.filter === item.value ? 'var(--text-primary)' : 'var(--bg-surface)', color: approvals.filter === item.value ? 'var(--text-inverse)' : 'var(--text-secondary)' }"
            @click="approvals.filter = item.value"
          >
            {{ item.label }}
          </button>
        </div>
        <button v-if="!isNative"
          v-for="item in approvals.visible"
          :key="item.id"
          class="v-card flex flex-col gap-[6px] text-left"
          :style="{ padding: approvals.selectedId === item.id ? '11px' : '12px', border: approvals.selectedId === item.id ? '2px solid var(--text-primary)' : '1px solid var(--border-default)' }"
          @click="approvals.select(item.id)"
        >
          <span class="v-row-between">
            <VocaStatus :meta="approvalStateMeta(item.state)" />
            <span class="v-tiny v-muted">{{ item.age }}</span>
          </span>
          <span class="v-mono" :style="{ fontSize: '11px' }">{{ item.title }}</span>
          <span class="v-tiny v-muted">{{ item.agent }}</span>
        </button>
      </div>

      <div v-if="selected && !isNative" class="v-grow v-card v-detail-panel">
        <div class="flex items-start gap-2">
          <span class="flex min-w-0 flex-1 flex-col gap-[2px]">
            <b class="text-[16px]">{{ selected.title }}</b>
            <span class="v-tiny v-muted">属于任务：{{ selected.taskId }} · {{ selected.agent }}</span>
          </span>
          <VocaStatus :meta="approvalStateMeta(selected.state)" />
          <VocaButton size="sm" variant="ghost" @click="router.push({ name: 'tasks' })">查看任务 <ArrowUpRight :size="12" /></VocaButton>
        </div>

        <div class="v-mono" :style="{ background: 'var(--bg-inset)', border: '1px solid var(--border-default)', borderRadius: '6px', padding: '10px', fontSize: '11px', lineHeight: 1.55 }">
          <div class="v-tiny v-muted">请求的操作</div>
          <div>{{ selected.target }}</div>
          <div class="v-muted" :style="{ marginTop: '4px' }">{{ selected.diff }}</div>
        </div>

        <div class="flex flex-col gap-[5px]">
          <span class="v-section-title" :style="{ fontSize: '11.5px' }">为什么需要你确认</span>
          <div v-for="risk in selected.risk" :key="risk" class="flex items-start gap-[7px]">
            <span class="v-small v-secondary" :style="{ lineHeight: 1.5 }">· {{ risk }}</span>
          </div>
        </div>

        <div class="v-banner is-neutral"><Shield :size="13" /><span class="v-small">授权作用域：{{ selected.scope }}</span></div>

        <div v-if="selected.state === 'pending'" class="flex flex-wrap items-center gap-2">
          <VocaButton @click="approvals.decide(selected.id, 'deny')">拒绝</VocaButton>
          <VocaButton variant="primary" @click="approvals.decide(selected.id, 'allow-once')">仅此一次允许</VocaButton>
          <span class="v-chip v-tiny" :style="{ border: '1px solid var(--border-default)', borderRadius: '6px', padding: '4px 9px', color: 'var(--text-muted)' }">
            本次任务内允许（需可靠撤销）· 规划中
          </span>
        </div>
        <div v-else class="v-banner is-neutral">该请求已有最终记录，不再提供批准入口。</div>

        <div class="flex flex-col gap-[7px]">
          <span class="v-section-title" :style="{ fontSize: '11.5px' }">记录</span>
          <div v-for="entry in selected.history" :key="`${entry.time}-${entry.title}`" class="flex items-start gap-[9px]">
            <span class="v-mono v-tiny v-muted" :style="{ width: '52px' }">{{ entry.time }}</span>
            <span class="flex flex-col gap-[1px]">
              <b class="v-small">{{ entry.title }}</b>
              <span class="v-tiny v-muted">{{ entry.detail }}</span>
            </span>
          </div>
        </div>
      </div>
      <div v-else class="v-grow v-card v-empty">
        <Shield :size="18" class="v-muted" />
        <span class="text-[15px] font-semibold">{{ isNative ? "本机审批尚未接入" : "当前没有审批记录" }}</span>
        <span class="v-small">{{ isNative ? "这里不会出现演示授权请求；运行时也不会根据此页面授予权限。" : "新的审批请求会显示在这里。" }}</span>
      </div>
    </div>
  </div>
</template>
