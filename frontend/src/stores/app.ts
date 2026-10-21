import { computed, ref, watchEffect } from "vue";
import { defineStore } from "pinia";
import { usePreferredDark } from "@vueuse/core";
import type { AppSettings, StatusTone } from "@/types";
import { useServices } from "@/services/adapters";

export interface ToastItem {
  id: number;
  text: string;
  tone: StatusTone;
}

const defaults: AppSettings = {
  theme: "system",
  density: "comfortable",
  grayscalePreview: false,
  reducedMotion: false,
  platform: "mac",
  closeBehavior: "window",
  concurrency: 3,
  autoRetry: false,
  logRetention: "30 天",
  notificationPrefs: { approval: true, failure: true, done: false, systemOnly: false },
};

/**
 * 应用级状态：外观设置、全局提示、首次启动状态。
 *
 * 说明：这里只保存“界面外壳”的状态；业务数据在各自的 Store 中。
 */
export const useAppStore = defineStore("app", () => {
  const services = useServices();
  const prefersDark = usePreferredDark();

  const settings = ref<AppSettings>({ ...defaults });
  const toasts = ref<ToastItem[]>([]);
  const onboardingDone = ref(false);
  const booted = ref(false);
  let toastId = 0;

  const resolvedTheme = computed<"light" | "dark">(() =>
    settings.value.theme === "system" ? (prefersDark.value ? "dark" : "light") : settings.value.theme,
  );

  const isMock = computed(() => services.mode === "mock");

  async function loadSettings(): Promise<void> {
    settings.value = await services.settings.getSettings();
    booted.value = true;
  }

  async function patchSettings(patch: Partial<AppSettings>): Promise<void> {
    settings.value = await services.settings.updateSettings(patch);
  }

  async function resetAppearance(): Promise<void> {
    settings.value = await services.settings.resetAppearance();
    toast("已恢复默认外观", "success");
  }

  /** 全局轻提示：最多同时 3 条，相同文案去重（与原型一致）。 */
  function toast(text: string, tone: StatusTone = "neutral"): void {
    if (toasts.value.some((item) => item.text === text)) return;
    const id = ++toastId;
    toasts.value = [...toasts.value, { id, text, tone }].slice(-3);
    window.setTimeout(() => {
      toasts.value = toasts.value.filter((item) => item.id !== id);
    }, 4200);
  }

  function dismissToast(id: number): void {
    toasts.value = toasts.value.filter((item) => item.id !== id);
  }

  function finishOnboarding(): void {
    onboardingDone.value = true;
  }

  // 把主题/密度/灰度/平台同步到 <html>/<body>，与原型 styles.css 的 data-* 一致
  watchEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolvedTheme.value;
    root.dataset.density = settings.value.density;
    root.dataset.platform = settings.value.platform;
    root.dataset.motion = settings.value.reducedMotion ? "reduced" : "normal";
    document.body.classList.toggle("is-grayscale", settings.value.grayscalePreview);
  });

  return {
    settings,
    toasts,
    booted,
    onboardingDone,
    resolvedTheme,
    isMock,
    loadSettings,
    patchSettings,
    resetAppearance,
    toast,
    dismissToast,
    finishOnboarding,
  };
});
