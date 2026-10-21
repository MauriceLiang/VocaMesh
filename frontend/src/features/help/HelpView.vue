<script setup lang="ts">
import { computed, ref } from "vue";
import { Search } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaTextarea from "@/components/ui/VocaTextarea.vue";
import { useAppStore } from "@/stores/app";

const app = useAppStore();
const query = ref("");
const active = ref("开始你的第一项任务");
const feedback = ref("");

const articles: Record<string, string[]> = {
  入门: ["开始你的第一项任务", "Agent 术语速查"],
  配置: ["安装与认证本地 CLI", "工具与角色的区别"],
  任务: ["暂停、停止与重试的区别", "审批如何工作", "部分成果与失败"],
  支持: ["反馈与问题", "快捷键"],
};

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return Object.entries(articles)
    .map(([group, items]) => [group, items.filter((item) => !q || item.toLowerCase().includes(q))] as const)
    .filter(([, items]) => items.length);
});

const bodies: Record<string, string[]> = {
  开始你的第一项任务: [
    "VocaMesh 的入口只有一个问题：“你想完成什么”。先描述任务，再决定用哪个 Agent 或团队执行。",
    "1. 检查可用的执行工具：左侧 → 本地工具。看到“已安装且可用”才能被执行。",
    "2. 创建 Agent 角色：只需名称、职责、执行工具三项即可保存。",
    "3. 描述任务并创建：确认工作空间与执行方式，创建后会先排队。",
    "4. 处理审批与异常：需要授权时任务会停下；失败时可只重试失败部分或跳过生成部分成果。",
  ],
  快捷键: ["⌘K / Ctrl+K：命令面板（↑↓ 选择，Enter 执行）", "Esc：关闭弹层 / 抽屉 / 菜单", "Enter：发送任务消息", "Tab：在控件之间移动焦点"],
};

function submit(): void {
  if (!feedback.value.trim()) {
    app.toast("请先填写反馈内容", "neutral");
    return;
  }
  app.toast("反馈已记录（本机演示，不会发送到服务器）", "success");
  feedback.value = "";
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="帮助与反馈" subtitle="不需要先理解 Agent 术语。这里有最短路径说明、概念对照和常见问题。">
      <template #actions>
        <label class="v-searchbox">
          <Search :size="13" />
          <input v-model="query" class="w-[120px] bg-transparent text-[12px] outline-none" placeholder="搜索帮助内容" aria-label="搜索帮助" />
        </label>
      </template>
    </PageHeader>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-col-nav v-card v-col" :style="{ padding: '8px', gap: '2px', overflow: 'auto' }">
        <template v-for="[group, items] in filtered" :key="group">
          <div class="v-tiny v-muted" :style="{ padding: '8px 8px 4px', fontWeight: 600, letterSpacing: '0.5px' }">{{ group }}</div>
          <button
            v-for="item in items"
            :key="item"
            class="rounded-[5px] px-2 py-[6px] text-left text-[11.5px]"
            :style="{ background: item === active ? 'var(--bg-subtle)' : 'transparent', fontWeight: item === active ? 600 : 400, color: item === active ? 'var(--text-primary)' : 'var(--text-secondary)' }"
            @click="active = item"
          >
            {{ item }}
          </button>
        </template>
      </div>

      <div class="v-grow v-card v-detail-panel">
        <h2 class="text-[18px] font-semibold" :style="{ letterSpacing: '-0.2px' }">{{ active }}</h2>
        <p v-for="(line, index) in bodies[active] ?? ['该篇内容在演示环境中为占位。正式内容由帮助中心提供。']" :key="index" class="v-small v-secondary" :style="{ lineHeight: 1.7, margin: 0 }">
          {{ line }}
        </p>
        <div class="v-divider" />
        <div class="flex flex-col gap-[7px]">
          <div class="flex items-center gap-2"><b class="v-small">反馈与问题</b><span class="v-tiny v-muted">不包含日志内容</span></div>
          <VocaTextarea v-model="feedback" :rows="2" aria-label="反馈内容" placeholder="哪里让你觉得不顺手？" />
          <div class="flex items-center gap-2"><span class="v-tiny v-muted">原型中提交只会显示演示提示</span><span class="v-spacer" /><VocaButton size="sm" variant="primary" @click="submit">提交反馈</VocaButton></div>
        </div>
      </div>
    </div>
  </div>
</template>
