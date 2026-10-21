import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { NotificationItem } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

/** 通知 Store：仅作为入口，审批操作仍在“待审批”。 */
export const useNotificationsStore = defineStore("notifications", () => {
  const services = useServices();
  const app = useAppStore();

  const items = ref<NotificationItem[]>([]);
  const filter = ref<"all" | "unread" | "approval">("all");
  const loading = ref(false);

  const unread = computed(() => items.value.filter((item) => item.unread).length);
  const visible = computed(() => {
    if (filter.value === "unread") return items.value.filter((item) => item.unread);
    if (filter.value === "approval") return items.value.filter((item) => item.kind === "blocked");
    return items.value;
  });

  async function load(): Promise<void> {
    loading.value = true;
    try {
      items.value = await services.notifications.listNotifications();
    } finally {
      loading.value = false;
    }
  }

  async function markRead(id: string): Promise<void> {
    await services.notifications.markRead(id);
    items.value = items.value.map((item) => (item.id === id ? { ...item, unread: false } : item));
  }

  async function markAllRead(): Promise<void> {
    await services.notifications.markAllRead();
    items.value = items.value.map((item) => ({ ...item, unread: false }));
    app.toast("全部通知已标为已读", "success");
  }

  async function dismiss(id: string): Promise<void> {
    await services.notifications.dismiss(id);
    items.value = items.value.filter((item) => item.id !== id);
  }

  return { items, filter, visible, unread, loading, load, markRead, markAllRead, dismiss };
});
