<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import { useEventListener } from "@vueuse/core";
import AppTitleBar from "./AppTitleBar.vue";
import SidebarNav from "./SidebarNav.vue";
import TopBar from "./TopBar.vue";
import CommandPalette from "./CommandPalette.vue";
import VocaToasts from "@/components/ui/VocaToasts.vue";
import { useApprovalsStore } from "@/stores/approvals";
import { useNotificationsStore } from "@/stores/notifications";
import { useTeamsStore } from "@/stores/teams";
import { useServices } from "@/services/adapters";

const route = useRoute();
const paletteOpen = ref(false);

const pageTitle = computed(() => (route.meta.title as string) ?? "VocaMesh");
const plain = computed(() => route.meta.plain === true);

const approvals = useApprovalsStore();
const notifications = useNotificationsStore();
const teams = useTeamsStore();
const services = useServices();

onMounted(async () => {
  // 外壳依赖的计数与基础数据，先加载一次；页面自身再补充自己的数据
  await Promise.all([approvals.load(), notifications.load(), teams.load()]);
});

useEventListener(window, "keydown", (event: KeyboardEvent) => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    paletteOpen.value = !paletteOpen.value;
  }
  if (event.key === "Escape") paletteOpen.value = false;
});

</script>

<template>
  <div class="v-app">
    <template v-if="plain">
      <RouterView />
    </template>
    <template v-else>
      <AppTitleBar :label="pageTitle" />
      <div class="v-body">
        <SidebarNav />
        <div class="v-main">
          <TopBar :page-title="pageTitle" :on-open-palette="() => (paletteOpen = true)" />
          <div v-if="services.mode === 'tauri'" class="v-banner is-neutral mx-4 mt-2" role="status">
            {{ services.sourceLabel }}
          </div>
          <div class="v-workspace">
            <RouterView v-slot="{ Component }">
              <component :is="Component" />
            </RouterView>
          </div>
        </div>
      </div>
      <CommandPalette v-model="paletteOpen" />
      <VocaToasts />
    </template>
  </div>
</template>
