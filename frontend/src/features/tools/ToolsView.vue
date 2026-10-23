<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Bot, Plug, RefreshCw, ShieldCheck, SquareTerminal } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import VocaDialog from "@/components/ui/VocaDialog.vue";
import VocaInput from "@/components/ui/VocaInput.vue";
import { useToolsStore } from "@/stores/tools";
import { executionReasonText, toolAuthMeta, toolDiscoveryMeta, toolIntegrationMeta, toolRuntimeMeta, toolVersionMeta } from "@/lib/status";

const tools = useToolsStore();
const pathOpen = ref(false);
const pathValue = ref("");
const pathTarget = ref<string>("");
const configOpen = ref(false);
const configText = ref("");

const metrics = computed(() => [
  { label: "已发现", value: tools.counts.discovered, tone: "var(--status-success)", icon: "check" },
  { label: "认证待确认", value: tools.counts.authPending, tone: "var(--status-blocked)", icon: "alert" },
  { label: "未安装", value: tools.counts.missing, tone: "var(--text-secondary)", icon: "plug" },
  { label: "执行受限", value: tools.counts.executionRestricted, tone: "var(--text-muted)", icon: "help" },
]);

let unsubscribe: (() => void) | undefined;
onMounted(async () => {
  unsubscribe = tools.subscribeScan();
  if (!tools.tools.length) await tools.load();
});
onUnmounted(() => unsubscribe?.());

function openPath(id?: string): void {
  pathTarget.value = id ?? tools.tools[0]?.id ?? "claude";
  const tool = tools.tools.find((item) => item.id === pathTarget.value);
  pathValue.value = tool?.discoveryStatus === "found" ? tool.path : "";
  pathOpen.value = true;
}

async function savePath(): Promise<void> {
  if (await tools.setPath(pathTarget.value, pathValue.value)) pathOpen.value = false;
}

function syncPathValue(): void {
  pathValue.value = tools.tools.find((tool) => tool.id === pathTarget.value)?.path ?? "";
}

function openConfig(id: string): void {
  const tool = tools.tools.find((item) => item.id === id);
  configText.value = JSON.stringify(
    { name: tool?.name, adapter: tool?.adapter, version: tool?.version, path: tool?.path, state: tool?.state,
      canonicalPath: tool?.canonicalPath, discoveryStatus: tool?.discoveryStatus, versionStatus: tool?.versionStatus,
      authState: tool?.authState, integrationStatus: tool?.integrationStatus, runtimeReady: tool?.runtimeReady,
      reasonCode: tool?.reasonCode, runtimeReasonCode: tool?.runtimeReasonCode, capabilities: tool?.capabilities },
    null,
    2,
  );
  configOpen.value = true;
}

function checkedAt(timestamp: number): string {
  return timestamp ? new Date(timestamp).toLocaleString() : tools.mode === "mock" ? "Mock 演示时间" : "尚未检测";
}

function sourceText(source: string | null): string {
  if (source === "manual") return "手动路径";
  if (source === "path") return "PATH";
  if (source === "standard-location") return "标准安装目录";
  return tools.mode === "mock" ? "Mock 来源" : "未知来源";
}

</script>

<template>
  <div class="v-content">
    <PageHeader title="本地工具" subtitle="查看这台电脑发现的 CLI、版本、适配器能力和当前运行条件。工具是运行环境，不是 Agent 身份。">
      <template #actions>
        <VocaButton :disabled="tools.loading || !!tools.verifyingToolId" @click="tools.load"><RefreshCw :size="13" />{{ tools.loading ? "扫描中…" : "重新扫描" }}</VocaButton>
        <VocaButton variant="primary" :disabled="tools.loading || !!tools.verifyingToolId || !tools.tools.length" @click="openPath()">手动指定路径</VocaButton>
      </template>
    </PageHeader>

    <div v-if="tools.mode === 'mock'" class="v-banner is-neutral" role="status">当前为浏览器 Mock 预览；不会扫描本机 CLI。</div>
    <div v-if="tools.scanning" class="v-tiny v-muted" role="status">{{ tools.scanMessage || "正在扫描本机工具…" }}</div>
    <div v-if="tools.error" class="v-banner is-blocked" role="alert">{{ tools.error }}</div>

    <div class="v-metrics">
      <div v-for="metric in metrics" :key="metric.label" class="v-metric">
        <span class="ib" :style="{ color: metric.tone }">
          <component :is="metric.icon === 'plug' ? Plug : metric.icon === 'alert' ? ShieldCheck : metric.icon === 'check' ? ShieldCheck : Bot" :size="14" />
        </span>
        <span class="flex flex-col gap-[1px]"><b class="v">{{ metric.value }}</b><span class="l">{{ metric.label }}</span></span>
      </div>
    </div>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-grow v-col" :style="{ gap: '8px', minWidth: 0, overflow: 'auto' }">
        <div v-for="tool in tools.tools" :key="tool.id" class="v-card v-panel" :style="{ gap: '9px' }">
          <div class="flex items-center gap-[10px]">
            <span class="grid h-[30px] w-[30px] place-items-center rounded-[7px]" :style="{ background: 'var(--bg-subtle)' }">
              <SquareTerminal :size="15" />
            </span>
            <span class="flex min-w-0 flex-1 flex-col gap-[1px]">
              <span class="flex items-center gap-[7px]">
                <b class="text-[13px]">{{ tool.name }}</b>
                <span class="v-mono v-tiny v-muted">{{ tool.version }}</span>
                <span v-if="tools.defaultToolId === tool.id" class="v-badge is-neutral">偏好工具</span>
              </span>
              <span class="v-mono v-tiny v-muted">适配器 {{ tool.adapter }} · {{ tool.path }}</span>
              <span class="v-tiny v-muted">来源 {{ sourceText(tool.source) }} · 最近检测 {{ checkedAt(tool.lastCheckedAt) }}</span>
            </span>
          </div>
          <div class="flex flex-wrap gap-[6px]">
            <VocaStatus :meta="toolDiscoveryMeta(tool.discoveryStatus)" />
            <VocaStatus :meta="toolVersionMeta(tool.versionStatus)" />
            <VocaStatus :meta="toolAuthMeta(tool.authState, tool.reasonCode)" />
            <VocaStatus :meta="toolIntegrationMeta(tool.integrationStatus)" />
            <VocaStatus :meta="toolRuntimeMeta(tool.runtimeReady, tool.runtimeReasonCode)" />
          </div>
          <div class="v-banner" :class="tool.reasonCode ? 'is-blocked' : 'is-neutral'" :style="{ padding: '6px 9px' }">
            <span class="v-tiny">{{ tool.note }} · {{ executionReasonText(tool.reasonCode) }}</span>
          </div>
          <div v-if="tool.runtimeReasonCode" class="v-tiny v-muted">Runtime 原因：{{ executionReasonText(tool.runtimeReasonCode) }}</div>
          <div class="flex flex-wrap items-center gap-[7px]">
            <VocaButton
              size="sm"
              :variant="tool.capabilities.authenticationProbeSupported ? 'primary' : 'secondary'"
              :disabled="!!tools.verifyingToolId && tools.verifyingToolId !== tool.id"
              :loading="tools.verifyingToolId === tool.id"
              @click="tools.verify(tool.id)"
            >
              {{ tools.verifyingToolId === tool.id ? "正在验证…" : tool.capabilities.authenticationProbeSupported ? "验证认证" : "重新检测" }}
            </VocaButton>
            <VocaButton size="sm" variant="ghost" :disabled="!tools.canSetPreference(tool)" @click="tools.setDefault(tool.id)">{{ tools.defaultToolId === tool.id ? "偏好工具 ✓" : "设为偏好" }}</VocaButton>
            <VocaButton size="sm" variant="ghost" @click="openConfig(tool.id)">查看配置</VocaButton>
            <VocaButton size="sm" variant="ghost" @click="openPath(tool.id)">手动指定路径</VocaButton>
          </div>
        </div>
      </div>

      <div class="v-col-rail v-card v-detail-panel">
        <span class="v-section-title">发现、能力与运行状态</span>
        <p class="v-tiny v-secondary" :style="{ margin: 0, lineHeight: 1.55 }">
          CLI 发现、版本、认证、适配器能力和 Runtime 就绪分别显示。认证未知不等于未登录；已登录也不代表 VocaMesh 当前允许执行。设为偏好只保存选择。
        </p>
        <div v-for="(step, index) in ['安装 CLI：在官方渠道安装后回到本页重新扫描。', 'Claude Code 登录状态仅通过固定的只读探测判断；其他 CLI 当前不读取登录状态。', '只有具备适配器且版本有效的工具能设为偏好；真实执行仍需 Runtime 全部检查通过。']" :key="index" class="flex items-start gap-[9px]">
          <span class="grid h-[18px] w-[18px] place-items-center rounded-full text-[10px] font-semibold" :style="{ background: 'var(--bg-subtle)' }">{{ index + 1 }}</span>
          <span class="v-tiny v-muted" :style="{ lineHeight: 1.5 }">{{ step }}</span>
        </div>
        <div class="v-banner is-neutral" :style="{ padding: '8px', alignItems: 'flex-start' }">
          <span class="v-tiny">普通扫描只发现 CLI 和版本，认证显示“尚未验证”；仅支持发现不等于安装失败。验证认证只执行固定只读命令，不发起模型请求。</span>
        </div>
        <span class="v-spacer" />
        <div class="v-divider" />
        <div class="flex items-center gap-2">
          <span class="v-tiny v-muted flex-1">需要更详细的步骤？</span>
          <RouterLink :to="{ name: 'help' }"><VocaButton size="sm">打开帮助</VocaButton></RouterLink>
        </div>
      </div>
    </div>

    <VocaDialog v-model="pathOpen" title="手动指定可执行文件路径" subtitle="保存后不会立即运行；重新检测会直接启动所选程序并传入固定 --version 参数，不经过 Shell。" :width="560">
      <div class="v-field"><span class="lbl">目标工具</span><select v-model="pathTarget" aria-label="目标工具" class="v-input" @change="syncPathValue"><option v-for="tool in tools.tools" :key="tool.id" :value="tool.id">{{ tool.name }}</option></select></div>
      <div class="v-field"><span class="lbl">可执行文件路径</span><VocaInput v-model="pathValue" aria-label="可执行文件路径" /></div>
      <template #footer>
        <VocaButton variant="ghost" @click="pathOpen = false">取消</VocaButton>
        <VocaButton variant="primary" @click="savePath">保存路径</VocaButton>
      </template>
    </VocaDialog>

    <VocaDialog v-model="configOpen" title="工具检测摘要" subtitle="显示本次结构化探测结果；不会读取认证配置或凭据文件。">
      <pre class="v-mono" :style="{ background: 'var(--bg-inset)', border: '1px solid var(--border-default)', borderRadius: '6px', padding: '10px', fontSize: '11px', whiteSpace: 'pre-wrap' }">{{ configText }}</pre>
      <template #footer><VocaButton @click="configOpen = false">关闭</VocaButton></template>
    </VocaDialog>
  </div>
</template>
