import type { NotificationItem } from "@/types";

/**
 * 通知服务。
 *
 * 用途：聚合任务事件与审批提醒。通知只是入口，审批操作仍在“待审批”。
 * 浏览器实现：MockNotificationService。
 * Tauri：当前返回空列表；原生通知和已读状态尚未接入。
 */
export interface NotificationService {
  /** 列出通知。调用时机：S15、顶栏未读角标。 */
  listNotifications(): Promise<NotificationItem[]>;

  /** 标记单条已读。 */
  markRead(id: string): Promise<void>;

  /** 全部标为已读。 */
  markAllRead(): Promise<void>;

  /** 移除通知（不删除底层的审批/任务记录）。 */
  dismiss(id: string): Promise<void>;

  /** 未读数（供角标）。 */
  unreadCount(): Promise<number>;
}
