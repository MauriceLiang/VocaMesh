<script setup lang="ts">
import { useRouter } from "vue-router";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import type { StatusTone } from "@/types";

const router = useRouter();

const cards: Array<{ code: string; title: string; description: string; tone: StatusTone; label: string; actions: Array<{ label: string; run: () => void }> }> = [
  { code: "E01", title: "未发现任何可运行工具", description: "工具显示“未安装 / 无法检测”，任务按钮禁用并说明原因。", tone: "neutral", label: "未检测到", actions: [{ label: "查看本地工具", run: () => router.push({ name: "tools" }) }] },
  { code: "E05", title: "下属执行失败 · 依赖阻塞", description: "失败节点红色，依赖节点琥珀“已阻塞”，无关任务继续。", tone: "failure", label: "执行失败", actions: [{ label: "打开失败任务", run: () => router.push({ name: "tasks" }) }] },
  { code: "E08", title: "用户点击暂停", description: "“等待暂停：已停止派发，当前步骤仍在运行”。", tone: "neutral", label: "等待暂停", actions: [{ label: "查看任务详情", run: () => router.push({ name: "tasks" }) }] },
  { code: "E10", title: "停止请求未确认", description: "“正在请求停止”与“已停止”分开；超时进入“状态待确认”。", tone: "unknown", label: "状态待确认", actions: [{ label: "查看历史", run: () => router.push({ name: "history" }) }] },
  { code: "E12", title: "并发资源已满", description: "任务排队并给出可解释原因，不静默丢弃。", tone: "neutral", label: "排队等待资源", actions: [{ label: "调整并发上限", run: () => router.push({ name: "settings" }) }] },
  { code: "E13", title: "多任务写入同一目录", description: "冲突提醒 + 工作空间边界 + 隔离选项。", tone: "blocked", label: "文件冲突风险", actions: [{ label: "查看工作空间", run: () => router.push({ name: "workspaces" }) }] },
  { code: "E15", title: "部分成果交付", description: "按完成 / 跳过 / 失败分组，标注缺失范围。", tone: "blocked", label: "部分完成", actions: [{ label: "查看历史记录", run: () => router.push({ name: "history" }) }] },
  { code: "E17", title: "无任务 · 无团队 · 无记录", description: "空状态提供明确下一步，而不是空白页。", tone: "neutral", label: "暂无内容", actions: [{ label: "创建工作台任务", run: () => router.push({ name: "dashboard" }) }] },
  { code: "E18", title: "窄窗 / 长文本 / 大量日志", description: "文本换行、区域独立滚动、日志可搜索复制。", tone: "neutral", label: "已截断显示", actions: [{ label: "打开日志", run: () => router.push({ name: "tasks" }) }] },
];
</script>

<template>
  <div class="v-content">
    <PageHeader title="异常与空状态合集" subtitle="同一个“失败弹窗”不能满足全部场景：每种状态都说明发生了什么、影响范围与下一步。" />
    <div class="grid grid-cols-3 gap-3" :style="{ flex: '1 1 auto', minHeight: 0, overflow: 'auto', alignContent: 'start' }">
      <div v-for="card in cards" :key="card.code" class="v-card v-panel" :style="{ gap: '7px' }">
        <div class="flex items-center gap-[7px]"><span class="v-mono v-tiny v-muted">{{ card.code }}</span><span class="v-spacer" /><VocaStatus :meta="{ label: card.label, tone: card.tone, icon: 'dot' }" /></div>
        <b class="text-[12.5px]">{{ card.title }}</b>
        <span class="v-tiny v-secondary" :style="{ lineHeight: 1.5 }">{{ card.description }}</span>
        <div class="flex flex-wrap items-center gap-[6px]">
          <VocaButton v-for="action in card.actions" :key="action.label" size="sm" @click="action.run()">{{ action.label }}</VocaButton>
        </div>
      </div>
    </div>
  </div>
</template>
