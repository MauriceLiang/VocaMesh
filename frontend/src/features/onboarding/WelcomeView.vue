<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { Bot, Plug, SquareTerminal } from "lucide-vue-next";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import { useAppStore } from "@/stores/app";
import { useToolsStore } from "@/stores/tools";
import { toolAuthMeta, toolDiscoveryMeta, toolStateMeta, toolVersionMeta } from "@/lib/status";
import { useServices } from "@/services/adapters";

const app = useAppStore();
const services = useServices();
const mockPreview = services.mode === "mock";
const tools = useToolsStore();
const router = useRouter();
const scanning = ref(false);

onMounted(async () => {
  if (!tools.tools.length) {
    scanning.value = true;
    await tools.load();
    scanning.value = false;
  }
});

async function rescan(): Promise<void> {
  scanning.value = true;
  await tools.load();
  scanning.value = false;
}

async function start(): Promise<void> {
  app.finishOnboarding();
  await router.push({ name: "dashboard" });
}
</script>

<template>
  <div class="flex h-screen flex-col" :style="{ background: 'var(--bg-canvas)' }">
    <div v-if="mockPreview" class="v-titlebar">
      <div class="v-lights"><span class="v-light r" /><span class="v-light y" /><span class="v-light g" /></div>
      <span class="v-small" :style="{ color: 'var(--text-muted)' }">VocaMesh — 首次启动</span>
      <span class="v-kbd" style="text-transform: uppercase; letter-spacing: 0.4px; border-style: dashed">mock 演示 · 不调用真实 CLI</span>
      <div class="drag" />
    </div>

    <div class="flex flex-1 flex-col items-center justify-center gap-[18px] px-10">
      <div class="flex items-center gap-[9px]">
        <span class="h-[26px] w-[26px] rounded-[7px]" :style="{ background: 'var(--text-primary)' }" />
        <b class="text-[16px]">VocaMesh</b>
        <span class="v-tiny v-muted">by Vocafect</span>
      </div>

      <div class="v-card" :style="{ width: '660px', maxWidth: '100%', padding: '28px', display: 'flex', flexDirection: 'column', gap: '16px' }">
        <div class="flex flex-col gap-[6px]">
          <h1 class="text-[25px] font-semibold" :style="{ margin: 0, letterSpacing: '-0.5px' }">欢迎使用 VocaMesh</h1>
          <p class="v-small v-secondary" :style="{ margin: 0, lineHeight: 1.65 }">
            {{ mockPreview ? "VocaMesh 在本机调用你已安装的 Agent 工具，帮你把一件工作交给多个 Agent 协作完成。这里不需要创建组织，也不需要输入任何 API Key。" : "VocaMesh 将本地保存 Agent、工作空间和团队配置。当前原生执行仍受安全策略限制，不会启动 Agent 任务。" }}
          </p>
        </div>

        <div class="v-card" :style="{ overflow: 'hidden' }">
          <div class="v-row-between" :style="{ padding: '8px 12px', background: 'var(--bg-inset)', fontSize: '11.5px' }">
            <b>{{ scanning ? "正在检查本机可用的执行工具…" : "本机可用的执行工具" }}</b>
            <span class="v-tiny v-muted">{{ scanning ? "扫描中" : mockPreview ? "上次扫描 3 秒前" : "状态来自本机探测" }}</span>
          </div>
          <div :style="{ padding: '0 12px' }">
            <div v-for="tool in tools.tools" :key="tool.id" class="flex items-center gap-[10px] border-t py-[9px]" :style="{ borderColor: 'var(--border-default)' }">
              <span class="grid h-[26px] w-[26px] place-items-center rounded-[6px]" :style="{ background: 'var(--bg-subtle)' }">
                <component :is="tool.id === 'claude' ? Bot : tool.id === 'codex' ? SquareTerminal : Plug" :size="13" />
              </span>
              <span class="flex min-w-0 flex-1 flex-col gap-[1px]">
                <span class="flex items-center gap-[7px]"><b class="text-[12.5px] font-medium">{{ tool.name }}</b><span class="v-mono v-tiny v-muted">{{ tool.version }}</span></span>
                <span class="v-mono v-tiny v-muted">{{ tool.path }}</span>
                  <span v-if="!mockPreview" class="v-tiny v-muted">{{ toolVersionMeta(tool.versionStatus).label }} · {{ toolAuthMeta(tool.authState).label }}</span>
              </span>
              <VocaStatus :meta="mockPreview ? toolStateMeta(tool.state) : toolDiscoveryMeta(tool.discoveryStatus)" />
            </div>
          </div>
        </div>

        <p class="v-tiny v-muted" :style="{ margin: 0, lineHeight: 1.6 }">
          检测只显示可执行文件、版本、认证探测能力和 Runtime 状态，不读取凭据内容；状态彼此独立，不会从“已发现”推断“可执行”。
        </p>

        <div class="flex items-center gap-2">
          <VocaButton :disabled="scanning" @click="rescan">{{ scanning ? "扫描中…" : "重新扫描" }}</VocaButton>
          <VocaButton variant="ghost" @click="start">跳过引导，先浏览</VocaButton>
          <span class="v-spacer" />
          <VocaButton variant="primary" @click="start">开始使用 VocaMesh</VocaButton>
        </div>
      </div>

      <p class="v-tiny v-muted">{{ mockPreview ? "下一步可选：创建第一个 Agent 角色（也可稍后在左侧“Agent 角色”中完成）" : "下一步可选：登记工作空间并创建 Agent 角色；这些配置不会授予执行权限。" }}</p>
    </div>
  </div>
</template>
