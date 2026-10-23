<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Cpu, Info, ShieldCheck, Sun, Bell } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaSwitch from "@/components/ui/VocaSwitch.vue";
import VocaSegmented from "@/components/ui/VocaSegmented.vue";
import VocaSelect from "@/components/ui/VocaSelect.vue";
import { useAppStore } from "@/stores/app";
import { useServices } from "@/services/adapters";
import type { AppSettings } from "@/types";
import { ServiceError } from "@/types";

const app = useAppStore();
const services = useServices();
const isNative = services.mode === "tauri";
const draftSettings = ref<AppSettings>({ ...app.settings });
const displaySettings = computed(() => isNative ? draftSettings.value : app.settings);
const saveState = ref<"saved" | "saving" | "failed">(app.settingsError ? "failed" : "saved");
const saveError = ref<string | null>(app.settingsError);

watch(() => app.settings, (settings) => {
  draftSettings.value = { ...settings, notificationPrefs: { ...settings.notificationPrefs } };
}, { deep: true });

watch(() => app.settingsError, (error) => {
  if (error) {
    saveError.value = error;
    saveState.value = "failed";
  } else if (saveError.value && saveState.value === "failed") {
    saveError.value = null;
    saveState.value = "saved";
  }
});

const sections = [
  { id: "appearance", label: "外观", icon: Sun },
  { id: "runtime", label: "执行与资源", icon: Cpu },
  { id: "notifications", label: "通知", icon: Bell },
  { id: "security", label: "权限与安全", icon: ShieldCheck },
  { id: "about", label: "关于", icon: Info },
] as const;

type SectionId = (typeof sections)[number]["id"];
const current = ref<SectionId>("appearance");

const themeItems = [
  { value: "system" as const, label: "跟随系统" },
  { value: "light" as const, label: "亮色" },
  { value: "dark" as const, label: "暗色" },
];

const densityItems = [
  { value: "comfortable" as const, label: "舒适" },
  { value: "compact" as const, label: "紧凑" },
];

const retentionItems = [
  { value: "7 天", label: "7 天" },
  { value: "30 天", label: "30 天" },
  { value: "90 天", label: "90 天" },
  { value: "永久保留", label: "永久保留" },
];

function setSection(value: SectionId): void {
  current.value = value;
}

async function patch(part: Partial<AppSettings>): Promise<void> {
  if (isNative) draftSettings.value = { ...draftSettings.value, ...part };
  saveState.value = "saving";
  saveError.value = null;
  try {
    if (isNative) {
      const { theme, density, grayscalePreview, reducedMotion } = draftSettings.value;
      await app.patchSettings({ theme, density, grayscalePreview, reducedMotion });
    } else {
      await app.patchSettings(part);
    }
    saveState.value = "saved";
  } catch (err) {
    saveState.value = "failed";
    saveError.value = err instanceof ServiceError ? err.message : "设置保存失败，请重试。";
    app.toast(saveError.value, "failure");
  }
}

async function retrySave(): Promise<void> {
  await patch({});
}

async function resetAppearance(): Promise<void> {
  if (isNative) {
    draftSettings.value = {
      ...draftSettings.value,
      theme: "system",
      density: "comfortable",
      grayscalePreview: false,
      reducedMotion: false,
    };
    await retrySave();
    return;
  }
  try {
    await app.resetAppearance();
    saveState.value = "saved";
    saveError.value = null;
  } catch (err) {
    saveState.value = "failed";
    saveError.value = err instanceof ServiceError ? err.message : "恢复默认外观失败，请重试。";
    app.toast(saveError.value, "failure");
  }
}

async function resetAll(): Promise<void> {
  await services.settings.resetDemoData();
  await app.loadSettings();
  app.toast("演示数据已重置", "success");
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="设置" subtitle="更改会立即生效；高风险选项会说明生效范围与恢复方式。" />

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-col-nav v-col" :style="{ gap: '2px' }">
        <button
          v-for="item in sections"
          :key="item.id"
          class="v-nav-item"
          :class="{ 'is-active': current === item.id }"
          @click="setSection(item.id)"
        >
          <component :is="item.icon" :size="14" />
          <span class="flex-1 text-left">{{ item.label }}</span>
        </button>
      </div>

      <div class="v-grow v-card v-detail-panel">
        <template v-if="current === 'appearance'">
          <span class="text-[15px] font-semibold">外观</span>

          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">主题</b><span class="v-tiny v-muted">跟随系统时随 macOS / Windows 切换；主色始终保持黑白灰。</span></span>
            <VocaSegmented :model-value="displaySettings.theme" :items="themeItems" @update:model-value="(value) => patch({ theme: value })" />
          </div>

          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">界面密度</b><span class="v-tiny v-muted">紧凑模式行高 30、控件 28、内边距 20。</span></span>
            <VocaSegmented :model-value="displaySettings.density" :items="densityItems" @update:model-value="(value) => patch({ density: value })" />
          </div>

          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">减少动态效果</b><span class="v-tiny v-muted">关闭过渡动画与滚动惯性，仅保留必要反馈。</span></span>
            <VocaSwitch :model-value="displaySettings.reducedMotion" label="减少动态效果" @update:model-value="(value) => patch({ reducedMotion: value })" />
          </div>

          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">灰度预览（可访问性走查）</b><span class="v-tiny v-muted">验证状态不只依赖颜色。</span></span>
            <VocaSwitch :model-value="displaySettings.grayscalePreview" label="灰度预览" @update:model-value="(value) => patch({ grayscalePreview: value })" />
          </div>

          <div v-if="!isNative" class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">窗口外框预览</b><span class="v-tiny v-muted">检查 macOS 与 Windows 标题栏控件安全区。</span></span>
            <VocaSegmented
              :model-value="app.settings.platform"
              :items="[{ value: 'mac', label: 'macOS' }, { value: 'windows', label: 'Windows' }]"
              @update:model-value="(value) => patch({ platform: value })"
            />
          </div>

          <div v-if="!isNative" class="v-divider" />
          <template v-if="!isNative">
          <span class="text-[15px] font-semibold">窗口与退出</span>
          <button
            v-for="option in [
              { value: 'window', title: '仅关闭窗口', text: '任务在后台继续运行；再次打开应用可回到同一任务。' },
              { value: 'app', title: '退出应用', text: '退出前提示如何处理运行中的任务：停止、等待或下次启动恢复。' },
            ]"
            :key="option.value"
            class="flex items-start gap-[9px] text-left"
            @click="patch({ closeBehavior: option.value as 'window' | 'app' })"
          >
            <span class="mt-[2px] grid h-[15px] w-[15px] place-items-center rounded-full border" :style="{ borderColor: app.settings.closeBehavior === option.value ? 'var(--text-primary)' : 'var(--border-strong)' }">
              <span v-if="app.settings.closeBehavior === option.value" class="h-[7px] w-[7px] rounded-full" :style="{ background: 'var(--text-primary)' }" />
            </span>
            <span class="flex flex-col gap-[1px]"><b class="v-small">{{ option.title }}</b><span class="v-tiny v-muted">{{ option.text }}</span></span>
          </button>
          <div class="v-banner is-neutral">原型说明：后台运行能力取决于核心进程的部署方式（当前仅模拟提示）。</div>
          </template>
          <div v-else class="v-banner is-neutral">Tauri 使用系统原生窗口控件。关闭窗口会停止本机运行；后台继续运行尚未支持。</div>
        </template>

        <template v-else-if="current === 'runtime'">
          <span class="text-[15px] font-semibold">执行与资源</span>
          <template v-if="isNative">
            <div class="flex items-center gap-3"><span class="v-small flex-1">本机并发</span><span class="v-badge is-neutral">1 个 Run</span></div>
            <div class="flex items-center gap-3"><span class="v-small flex-1">自动重试</span><span class="v-badge is-neutral">尚未支持</span></div>
            <div class="flex items-center gap-3"><span class="v-small flex-1">日志保留</span><span class="v-badge is-neutral">本机数据库保留</span></div>
            <div class="v-banner is-neutral">这些选项由当前本机 Runtime 的实际能力决定，不提供无效的可编辑开关。</div>
          </template>
          <template v-else>
          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">并发上限</b><span class="v-tiny v-muted">达到上限时新任务排队并说明原因，不静默丢弃。</span></span>
            <div class="flex items-center gap-[6px]">
              <VocaButton size="sm" @click="patch({ concurrency: Math.max(1, app.settings.concurrency - 1) })">−</VocaButton>
              <span class="v-mono" :style="{ width: '26px', textAlign: 'center' }">{{ app.settings.concurrency }}</span>
              <VocaButton size="sm" @click="patch({ concurrency: Math.min(6, app.settings.concurrency + 1) })">+</VocaButton>
            </div>
          </div>
          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">自动重试</b><span class="v-tiny v-muted">仅当能证明尚未进入真实执行时允许；默认关闭。</span></span>
            <VocaSwitch :model-value="app.settings.autoRetry" label="自动重试" @update:model-value="(value) => patch({ autoRetry: value })" />
          </div>
          <div class="flex items-center gap-3">
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">日志保留</b><span class="v-tiny v-muted">Run 日志与产物保留在本机。</span></span>
            <VocaSelect :model-value="app.settings.logRetention" :items="retentionItems" aria-label="日志保留" @update:model-value="(value) => patch({ logRetention: value as AppSettings['logRetention'] })" />
          </div>
          </template>
        </template>

        <template v-else-if="current === 'notifications'">
          <span class="text-[15px] font-semibold">通知</span>
          <div v-if="isNative" class="v-banner is-neutral">本机通知事件源尚未接入；当前没有真实通知，也不会生成演示消息。</div>
          <template v-else>
          <div
            v-for="row in [
              { key: 'approval', title: '审批请求', text: '有 Agent 等待你决定时提醒。' },
              { key: 'failure', title: '任务失败或阻塞', text: '执行失败、依赖阻塞、状态待确认。' },
              { key: 'done', title: '任务完成', text: '任务完成后提醒一次。' },
              { key: 'systemOnly', title: '仅显示系统通知', text: '关闭应用内未读标记。' },
            ]"
            :key="row.key"
            class="flex items-center gap-3"
          >
            <span class="flex flex-1 flex-col gap-[2px]"><b class="v-small">{{ row.title }}</b><span class="v-tiny v-muted">{{ row.text }}</span></span>
            <VocaSwitch
              :model-value="app.settings.notificationPrefs[row.key as keyof AppSettings['notificationPrefs']]"
              :label="row.title"
              @update:model-value="(value) => patch({ notificationPrefs: { ...app.settings.notificationPrefs, [row.key]: value } })"
            />
          </div>
          </template>
        </template>

        <template v-else-if="current === 'security'">
          <span class="text-[15px] font-semibold">权限与安全</span>
          <div class="flex items-center gap-3"><span class="v-small flex-1">默认审批策略</span><span class="v-badge is-neutral">默认不允许</span></div>
          <div class="flex items-center gap-3"><span class="v-small flex-1">授权作用域</span><VocaButton size="sm" @click="$router.push({ name: 'approvals' })">查看审批记录</VocaButton></div>
          <div class="flex items-center gap-3"><span class="v-small flex-1">凭据</span><span class="v-badge is-neutral">仅本地 · 不保存 Key</span></div>
          <div class="v-banner is-neutral">{{ isNative ? "Agent 权限与工作空间范围当前仅保存为配置草稿，不授予 Runtime 能力；managed policy 未核验前真实执行保持阻断。" : "企业版预留：管理员策略、审批角色与审计导出。当前原型不包含多租户管理。" }}</div>
        </template>

        <template v-else>
          <span class="text-[15px] font-semibold">关于</span>
          <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">产品</span><span>VocaMesh · by Vocafect</span></div>
          <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">版本</span><span class="v-mono">{{ isNative ? "VocaMesh 0.1.0 · 原生桌面" : "frontend 0.1.0（Mock 阶段）" }}</span></div>
          <div class="flex gap-[10px] text-[11.5px]"><span class="v-muted" :style="{ width: '96px' }">数据</span><span>{{ isNative ? "Agent、工作空间、团队与 Task/Run 保存在本机 SQLite；浏览器 Mock 种子不接入桌面。" : "全部为虚构演示数据，不调用真实 CLI 或文件系统" }}</span></div>
          <div class="flex flex-wrap items-center gap-2">
            <VocaButton v-if="!isNative" size="sm" @click="resetAll">重置演示数据</VocaButton>
            <VocaButton size="sm" variant="ghost" @click="app.toast('快捷键：⌘K 命令面板 · Esc 关闭弹层 · Enter 发送消息', 'neutral')">查看快捷键</VocaButton>
          </div>
        </template>

        <div v-if="saveError" class="v-banner is-failure"><span class="flex-1">{{ saveError }}</span><VocaButton size="sm" @click="retrySave">重试保存</VocaButton></div>
        <span class="v-spacer" />
        <div class="flex items-center gap-2">
          <span class="v-tiny v-muted">{{ saveState === 'saving' ? '保存中…' : saveState === 'failed' ? '保存失败' : '更改已保存' }}</span>
          <span class="v-spacer" />
          <VocaButton size="sm" variant="ghost" @click="resetAppearance">恢复默认外观</VocaButton>
        </div>
      </div>
    </div>
  </div>
</template>
