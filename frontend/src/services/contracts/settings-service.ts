import type { AppSettings } from "@/types";

/**
 * 应用设置服务。
 *
 * 用途：外观、密度、窗口行为、并发、日志保留、通知偏好。
 * 当前实现：MockSettingsService（内存 + localStorage 可选）。
 * 未来实现：Rust Core 的设置持久化；窗口行为需与桌面进程能力对齐。
 */
export interface SettingsService {
  /** 读取设置。调用时机：应用启动、进入 S13。 */
  getSettings(): Promise<AppSettings>;

  /** 局部更新设置。错误：`settings.invalid`。 */
  updateSettings(patch: Partial<AppSettings>): Promise<AppSettings>;

  /** 重置为默认（仅外观相关）。 */
  resetAppearance(): Promise<AppSettings>;

  /** 重置全部演示数据（Mock 专用；正式实现应改为“清除本地缓存”并二次确认）。 */
  resetDemoData(): Promise<void>;
}
