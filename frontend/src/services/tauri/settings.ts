import type { AppSettings } from "@/types";
import { ServiceError } from "@/types";
import type { SettingsService } from "@/services/contracts";
import { invokeNative } from "./ipc";

interface AppearanceSettings {
  theme: AppSettings["theme"];
  density: AppSettings["density"];
  grayscalePreview: boolean;
  reducedMotion: boolean;
}

function platform(): AppSettings["platform"] {
  return /Macintosh|Mac OS X/i.test(navigator.userAgent) ? "mac" : "windows";
}

function completeSettings(appearance: AppearanceSettings): AppSettings {
  return {
    ...appearance,
    platform: platform(),
    closeBehavior: "app",
    concurrency: 1,
    autoRetry: false,
    logRetention: "永久保留",
    notificationPrefs: { approval: false, failure: false, done: false, systemOnly: true },
  };
}

const unsupportedKeys = ["platform", "closeBehavior", "concurrency", "autoRetry", "logRetention", "notificationPrefs"] as const;

export function createTauriSettingsService(): SettingsService {
  async function getAppearance(): Promise<AppearanceSettings> {
    return invokeNative<AppearanceSettings>("get_appearance_settings");
  }
  return {
    async getSettings() { return completeSettings(await getAppearance()); },
    async updateSettings(patch) {
      if (unsupportedKeys.some((key) => Object.hasOwn(patch, key))) {
        throw new ServiceError("该设置尚未由本机运行时支持", "feature.not-implemented");
      }
      const current = await getAppearance();
      const settings: AppearanceSettings = {
        theme: patch.theme ?? current.theme,
        density: patch.density ?? current.density,
        grayscalePreview: patch.grayscalePreview ?? current.grayscalePreview,
        reducedMotion: patch.reducedMotion ?? current.reducedMotion,
      };
      return completeSettings(await invokeNative<AppearanceSettings>("update_appearance_settings", { settings }));
    },
    async resetAppearance() {
      return completeSettings(await invokeNative<AppearanceSettings>("update_appearance_settings", {
        settings: { theme: "system", density: "comfortable", grayscalePreview: false, reducedMotion: false },
      }));
    },
    async resetDemoData() {
      throw new ServiceError("本机数据目录不支持重置演示数据", "feature.not-implemented");
    },
  };
}
