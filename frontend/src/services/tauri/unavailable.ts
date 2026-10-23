import type { ApprovalService, NotificationService } from "@/services/contracts";
import { ServiceError } from "@/types";

const unavailable = () => new ServiceError("该功能尚未接入本机后端", "feature.not-implemented");

export function createUnavailableApprovalService(): ApprovalService {
  return {
    async listApprovals() { return []; },
    async getApproval() { throw new ServiceError("当前没有真实审批记录", "approval.not-found"); },
    async decide() { throw unavailable(); },
    async canGrantTaskScope() { return false; },
    subscribe() { return () => undefined; },
  };
}

export function createEmptyNativeNotificationService(): NotificationService {
  return {
    async listNotifications() { return []; },
    async markRead() { throw unavailable(); },
    async markAllRead() { throw unavailable(); },
    async dismiss() { throw unavailable(); },
    async unreadCount() { return 0; },
  };
}
