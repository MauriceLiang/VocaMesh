import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import SettingsView from "@/features/settings/SettingsView.vue";
import { ServiceError, type AppSettings } from "@/types";
import { __setServicesForTesting, createServices } from "@/services/adapters";
import { useAppStore } from "@/stores/app";

const initialSettings: AppSettings = {
  theme: "system",
  density: "comfortable",
  grayscalePreview: false,
  reducedMotion: false,
  platform: "mac",
  closeBehavior: "app",
  concurrency: 1,
  autoRetry: false,
  logRetention: "永久保留",
  notificationPrefs: { approval: false, failure: false, done: false, systemOnly: true },
};

describe("native appearance settings", () => {
  afterEach(() => __setServicesForTesting(null));

  it("keeps the user's draft visible and offers retry without claiming a failed save succeeded", async () => {
    const services = createServices("tauri");
    const updateSettings = vi.fn()
      .mockRejectedValueOnce(new ServiceError("本机设置暂不可用", "storage.write-failed"))
      .mockResolvedValue({ ...initialSettings, theme: "dark" });
    services.settings = {
      getSettings: vi.fn().mockResolvedValue(initialSettings),
      updateSettings,
      resetAppearance: vi.fn().mockResolvedValue(initialSettings),
      resetDemoData: vi.fn(),
    };
    const pinia = createPinia();
    setActivePinia(pinia);
    __setServicesForTesting(services);
    await useAppStore().loadSettings();

    const wrapper = mount(SettingsView, { global: { plugins: [pinia] } });
    await wrapper.findAll("button").find((button) => button.text() === "暗色")!.trigger("click");
    await flushPromises();

    expect(updateSettings).toHaveBeenCalledWith({
      theme: "dark", density: "comfortable", grayscalePreview: false, reducedMotion: false,
    });
    expect(wrapper.text()).toContain("本机设置暂不可用");
    expect(wrapper.text()).toContain("保存失败");
    expect(wrapper.findAll("button").find((button) => button.text() === "暗色")?.attributes("aria-pressed")).toBe("true");

    const retry = wrapper.findAll("button").find((button) => button.text().includes("重试保存"));
    expect(retry).toBeDefined();
    await retry!.trigger("click");
    await flushPromises();
    expect(updateSettings).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain("更改已保存");
    expect(wrapper.text()).not.toContain("本机设置暂不可用");
  });
});
